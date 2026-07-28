use crate::server::ServerItem;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn expand_private_key_path(path: &str) -> PathBuf {
    path.strip_prefix("~/")
        .and_then(|relative| dirs::home_dir().map(|home| home.join(relative)))
        .unwrap_or_else(|| PathBuf::from(path))
}

fn private_key_requires_agent(path: &Path) -> bool {
    #[cfg(unix)]
    {
        return std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o077 != 0)
            .unwrap_or(false);
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

fn add_ssh_arguments(command: &mut Command, server: &ServerItem, private_key: Option<&Path>) {
    command.arg("-p").arg(server.port.to_string());
    if let Some(proxy_command) = &server.proxy_command {
        command
            .arg("-o")
            .arg(format!("ProxyCommand={proxy_command}"));
    }
    if let Some(private_key) = private_key {
        command.arg("-i").arg(private_key);
    }
    command.arg(format!("{}@{}", server.username, server.ip));
}

pub fn ssh_login(server: &ServerItem) {
    if let Some(password) = &server.password {
        // Use sshpass for password-based login
        let ssh_cmd = format!(
            "sshpass -p '{}' ssh {}@{} -p {}",
            "*".repeat(password.len()), server.username, server.ip, server.port
        );
        println!("Executing: {}", ssh_cmd);

        let mut command = Command::new("sshpass");
        command.arg("-p").arg(password).arg("ssh");
        add_ssh_arguments(&mut command, server, None);
        command
            .spawn()
            .expect("Failed to start SSH session with password")
            .wait()
            .expect("SSH process failed");
    } else {
        // Use key-based login
        let private_key = server.private_key.as_deref().map(expand_private_key_path);
        if private_key.as_deref().is_some_and(private_key_requires_agent) {
            let private_key = private_key.as_deref().unwrap();
            let ssh_cmd = format!(
                "ssh {}@{} -p {} via an ephemeral ssh-agent",
                server.username, server.ip, server.port
            );
            println!("Executing: {}", ssh_cmd);

            let mut command = Command::new("ssh-agent");
            command
                .arg("sh")
                .arg("-c")
                .arg("ssh-add - < \"$1\" && shift && exec \"$@\"")
                .arg("lazyssh")
                .arg(private_key)
                .arg("ssh");
            add_ssh_arguments(&mut command, server, None);
            command
                .spawn()
                .expect("Failed to start ssh-agent; make sure OpenSSH client tools are installed")
                .wait()
                .expect("SSH process failed");
            return;
        }

        let ssh_cmd = if let Some(private_key) = &private_key {
            format!(
                "ssh {}@{} -p {} -i {}",
                server.username,
                server.ip,
                server.port,
                private_key.display()
            )
        } else {
            format!("ssh {}@{} -p {}", server.username, server.ip, server.port)
        };
        println!("Executing: {}", ssh_cmd);

        let mut command = Command::new("ssh");
        add_ssh_arguments(&mut command, server, private_key.as_deref());
        command
            .spawn()
            .expect("Failed to start SSH session with key")
            .wait()
            .expect("SSH process failed");
    }
}

#[cfg(test)]
mod tests {
    use super::{add_ssh_arguments, expand_private_key_path, private_key_requires_agent};
    use crate::server::ServerItem;
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn adds_identity_file_without_shell_quotes() {
        let identity_file =
            "/media/wq/d/appdata/nextcloud/备忘存档/有为雷风/雷风社区1.pem";
        let server = ServerItem::new(
            "other",
            false,
            "server",
            "114.55.243.216",
            "root",
            22,
            Some(identity_file.to_string()),
            None,
            None,
        );
        let mut command = Command::new("ssh");

        add_ssh_arguments(&mut command, &server, Some(Path::new(identity_file)));

        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "-p".to_string(),
                "22".to_string(),
                "-i".to_string(),
                identity_file.to_string(),
                "root@114.55.243.216".to_string(),
            ]
        );
    }

    #[test]
    fn adds_proxy_command_without_unknown_identity_file() {
        let proxy_command = "ssh -p 6223 -W %h:%p root@101.37.124.230";
        let server = ServerItem::new(
            "other",
            false,
            "qlwy-db1",
            "192.168.63.45",
            "root",
            20211,
            None,
            None,
            Some(proxy_command.to_string()),
        );
        let mut command = Command::new("ssh");

        add_ssh_arguments(&mut command, &server, None);

        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "-p".to_string(),
                "20211".to_string(),
                "-o".to_string(),
                format!("ProxyCommand={proxy_command}"),
                "root@192.168.63.45".to_string(),
            ]
        );
        assert!(!args.iter().any(|arg| arg == "-i" || arg == "unknown"));
    }

    #[test]
    fn expands_home_relative_private_key_path() {
        let home = dirs::home_dir().expect("home directory should be available");

        assert_eq!(
            expand_private_key_path("~/.ssh/id_ed25519"),
            home.join(".ssh/id_ed25519")
        );
    }

    #[cfg(unix)]
    #[test]
    fn detects_overly_open_private_key_permissions() {
        use std::fs::{self, File};
        use std::os::unix::fs::PermissionsExt;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lazyssh-key-{unique}"));
        File::create(&path).expect("temporary key file should be created");

        for mode in [0o400, 0o600] {
            fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                .expect("permissions should be set");
            assert!(!private_key_requires_agent(&path));
        }

        for mode in [0o644, 0o666] {
            fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                .expect("permissions should be set");
            assert!(private_key_requires_agent(&path));
        }

        fs::remove_file(path).expect("temporary key file should be removed");
    }
}
