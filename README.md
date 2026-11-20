An ssh server manages tui tools.

![demo.gif](demo.gif)

## Install

### Use [Cargo](https://www.rust-lang.org/tools/install)

```bash
cargo install lazyssh
```

### Use Homebrew

```bash
brew install KURANADO2/lazyssh/lazyssh
```

## Usage

Simply run the `lazyssh` command in the terminal（It is recommended to add a command alias for `lazyssh`, such as `s`）,
and the TUI program will list all remote servers. You can select a server using your mouse or keyboard, double-click or
press Enter to log in to the server. All server information comes from the `~/.ssh/config` file.

## Choose an SSH client

By default, `lazyssh` starts the system `ssh` executable. Set `LAZYSSH_SSH` to use another
OpenSSH-compatible client, such as [trzsz-ssh (`tssh`)](https://github.com/trzsz/trzsz-ssh):

```bash
export LAZYSSH_SSH=tssh
lazyssh
```

You can also set it for a single launch with `LAZYSSH_SSH=tssh lazyssh`, or provide an
absolute executable path (for example, `LAZYSSH_SSH=/usr/local/bin/tssh`). This variable
specifies the executable **only**, not additional command-line arguments. Unset it (or
set it to an empty value) to use `ssh` again. Shell aliases such as `alias ssh='tssh'`
do not affect `lazyssh` because it starts the executable directly.

The selected SSH client is used for normal logins, `sshpass` password logins, and
logins using a temporary `ssh-agent`. If you use password authentication, the chosen
client must support the prompt behavior required by `sshpass`.

For `rz` / `sz` file transfers with `tssh`, install `lrzsz` on the remote server and
enable ZMODEM in your local `~/.ssh/config`:

```sshconfig
Host *
    #!! EnableZmodem yes
```

The `#!!` prefix lets OpenSSH ignore the `tssh`-specific setting. For more details,
see [the tssh documentation](https://trzsz.github.io/tssh.html). `ProxyCommand`
values from `~/.ssh/config` are passed through unchanged; any literal `ssh` inside
such a command is not rewritten.

## Shortcut

| Key                | Desc                        |
|--------------------|-----------------------------|
| Mouse click        | Select server               |
| Mouse scrolling    | Move down or up             |
| j/↓                | Move down                   |
| k/↑                | Move up                     |
| g/Home             | Move to top                 |
| G/End              | Move to bottom              |
| /                  | Enter search mode           |
| Ctrl+j/k or ↑/↓    | Move down/up in search mode |
| Backspace          | Delete search query chars   |
| Esc                | Exit search mode            |
| Double click/Enter | Perform SSH login           |
| z                  | Toggle all groups           |
| q                  | Exit                        |

## `~/.ssh/config` file Example

### Using public and private keys(Recommended)

```
Host Tencent ubuntu server
    HostName 49.235.30.166
    User root
    Port 22
    PreferredAuthentications publickey
    IdentityFile ~/.ssh/keys/ubuntu
```

On Unix systems, if an `IdentityFile` is accessible by the group or other users, `lazyssh` automatically loads it into
an isolated, temporary `ssh-agent` instead of passing it to `ssh -i`. This avoids creating another copy of the key, and
the agent exits when the SSH session ends. This behavior requires `ssh-agent` and `ssh-add`. It bypasses OpenSSH's file
permission check but does not make the original key file secure.

### Using a proxy command

`lazyssh` passes a host's complete `ProxyCommand` to OpenSSH, including `%h` and `%p` tokens:

```
Host qlwy-db1
    HostName 192.168.63.45
    Port 20211
    User root
    ProxyCommand ssh -p 6223 -W %h:%p root@101.37.124.230
```

If `IdentityFile` is omitted, OpenSSH uses its normal default keys and agent identities.

### Using password(Not recommended)

- Define password with `#: Password`, please make sure that the `sshpass` command is installed.

```
Host Media server
    HostName 49.235.30.205
    User root
    Port 22
    #: Password 123456
```

### Define grouping

- If you want to group servers, define group name with `#: Group`.

```
#: Group Personal servers
Host Tencent ubuntu
    HostName 49.235.30.166
    User root
    Port 22
    PreferredAuthentications publickey
    IdentityFile ~/.ssh/keys/ubuntu
Host Media server
    HostName 49.235.30.205
    User root
    Port 22
    #: Password 123456
Host Storage server
    HostName 49.235.30.206
    User root
    Port 22
    PreferredAuthentications publickey
    IdentityFile ~/.ssh/keys/storage_server
#: Group Dev
Host k8s_master
    HostName 192.168.19.200
    User root
    Port 22
    PreferredAuthentications publickey
    IdentityFile ~/.ssh/keys/k8s_master
Host dev_node1
    HostName 192.168.20.21
    User root
    Port 22
    PreferredAuthentications publickey
    IdentityFile ~/.ssh/keys/dev_node1
Host dev_node2
    HostName 192.168.20.34
    User root
    Port 22
    PreferredAuthentications publickey
    IdentityFile ~/.ssh/keys/dev_node2
#: Group Product
Host product_node1
    HostName 192.168.10.10
    User root
    Port 22
    PreferredAuthentications publickey
    IdentityFile ~/.ssh/keys/product_node1
Host product_node2
    HostName 192.168.10.13
    User root
    Port 22
    PreferredAuthentications publickey
    IdentityFile ~/.ssh/keys/product_node2
```

The `~/.ssh` file directory structure is as follows:

```
$ tree ~/.ssh
/Users/jing/.ssh
├── config
├── keys
│   ├── ubuntu
│   ├── ubuntu.pub
│   ├── storage_server
│   ├── storage_server.pub
│   ├── k8s_master
│   ├── k8s_master.pub
│   ├── dev_node1
│   ├── dev_node1.pub
│   ├── dev_node2
│   ├── dev_node2.pub
│   ├── product_node1
│   ├── product_node1.pub
│   ├── product_node2
│   └── product_node2.pub
└── known_hosts
```

## Tips

- You can use `ssh-keygen -t rsa -b 4096 -C youremail@xxx.com` to generate the private and public key. Use
  `ssh-copy-id -i xxx.pub -p 22 yourusername@x.x.x.x` to send the public key to the remote server.
- You can log in to multiple remote servers using one pair of public and private keys.
- The `Host` value in the `~/.ssh/config` file can be set to non-ASCII characters, so you can type Chinese, Japanese,
  Korean, etc.
- You can upload your `~/.ssh` folder to a git **private** repository to make it easy to synchronize configurations
  across multiple machines.
