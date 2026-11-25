use crate::server::ServerItem;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn resolve_ssh_program(configured: Option<&str>) -> &str {
    match configured {
        Some(program) if !program.trim().is_empty() => program,
        _ => "ssh",
    }
}

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

fn password_login_command(
    ssh_program: &str,
    server: &ServerItem,
    password: &str,
) -> (Command, String) {
    let destination = format!("{}@{} -p {}", server.username, server.ip, server.port);
    let program_name = Path::new(ssh_program)
        .file_name()
        .and_then(|name| name.to_str());
    let (mut command, display) = if matches!(program_name, Some("tssh" | "tssh.exe")) {
        let mut command = Command::new(ssh_program);
        if !password.is_empty() {
            command.arg("-o").arg(format!("Password={password}"));
        }
        (command, format!("{ssh_program} {destination}"))
    } else {
        let mut command = Command::new("sshpass");
        command.arg("-p").arg(password).arg(ssh_program);
        let masked_password = "*".repeat(password.len());
        (
            command,
            format!("sshpass -p '{masked_password}' {ssh_program} {destination}"),
        )
    };
    add_ssh_arguments(&mut command, server, None);
    (command, display)
}

pub fn ssh_login(server: &ServerItem) {
    let configured_ssh = std::env::var("LAZYSSH_SSH").ok();
    let ssh_program = resolve_ssh_program(configured_ssh.as_deref());
    if let Some(password) = &server.password {
        let (mut command, ssh_cmd) = password_login_command(ssh_program, server, password);
        println!("Executing: {}", ssh_cmd);

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
                "{} {}@{} -p {} via an ephemeral ssh-agent",
                ssh_program, server.username, server.ip, server.port
            );
            println!("Executing: {}", ssh_cmd);

            let mut command = Command::new("ssh-agent");
            command
                .arg("sh")
                .arg("-c")
                .arg("ssh-add - < \"$1\" && shift && exec \"$@\"")
                .arg("lazyssh")
                .arg(private_key)
                .arg(ssh_program);
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
                "{} {}@{} -p {} -i {}",
                ssh_program,
                server.username,
                server.ip,
                server.port,
                private_key.display()
            )
        } else {
            format!(
                "{} {}@{} -p {}",
                ssh_program, server.username, server.ip, server.port
            )
        };
        println!("Executing: {}", ssh_cmd);

        let mut command = Command::new(ssh_program);
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
    use super::{
        add_ssh_arguments, expand_private_key_path, password_login_command,
        private_key_requires_agent, resolve_ssh_program,
    };
    use crate::server::ServerItem;
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn defaults_to_ssh_and_accepts_a_custom_ssh_executable() {
        assert_eq!(resolve_ssh_program(None), "ssh");
        assert_eq!(resolve_ssh_program(Some("")), "ssh");
        assert_eq!(resolve_ssh_program(Some("   ")), "ssh");
        assert_eq!(resolve_ssh_program(Some("tssh")), "tssh");
        assert_eq!(
            resolve_ssh_program(Some("/usr/local/bin/tssh")),
            "/usr/local/bin/tssh"
        );
    }

    #[test]
    fn passes_the_saved_password_to_the_selected_client() {
        let password = r#"fixture #=' "密码"$"#;
        let proxy = "ProxyCommand=ssh -W %h:%p jump";
        let server = ServerItem::new(
            "other",
            false,
            "fixture",
            "example.org",
            "user",
            2222,
            None,
            Some(password.to_string()),
            Some("ssh -W %h:%p jump".to_string()),
        );
        let password_option = format!("Password={password}");
        let common_args = ["-p", "2222", "-o", proxy, "user@example.org"];

        for ssh_program in ["tssh", "/usr/local/bin/tssh", "tssh.exe"] {
            let (command, display) = password_login_command(ssh_program, &server, password);
            assert_eq!(command.get_program(), ssh_program);
            assert!(command
                .get_args()
                .eq(["-o", &password_option].into_iter().chain(common_args)));
            assert!(!display.contains(password));
        }

        let (command, _) = password_login_command("tssh", &server, "");
        assert!(command.get_args().eq(common_args));

        for ssh_program in ["ssh", "/usr/local/bin/custom-ssh"] {
            let (command, display) = password_login_command(ssh_program, &server, password);
            assert_eq!(command.get_program(), "sshpass");
            assert!(command
                .get_args()
                .eq(["-p", password, ssh_program].into_iter().chain(common_args)));
            assert!(!display.contains(password));
        }
    }

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
