# lview

A TUI for viewing logs on remote Linux machines over SSH. It automatically detects system logs, Docker containers, `/opt` applications, and the systemd journal.

## Installation

### macOS (Homebrew)

```bash
brew tap janvete/tools
brew install lview
```

### Debian / Ubuntu (.deb)

Download the latest `.deb` from [GitHub Releases](https://github.com/janvete/lview/releases) and install it:

```bash
sudo apt install ./lview_*.deb
# or
sudo dpkg -i ./lview_*.deb
```

### From source

```bash
cargo install --git https://github.com/janvete/lview
```

## Usage

```bash
lview ssh -p22 root@192.168.53.3
```

All arguments after `ssh` are passed directly to the system `ssh` command, so your `~/.ssh/config`, SSH agent, and keys are used automatically.

## Controls

### Log picker

| Key | Action |
|-----|--------|
| `j` / `k` or `↓` / `↑` | move through the list |
| `Enter` | open the selected log |
| `/` | fuzzy filter the log list |
| `r` | reload the log list |
| `q` | quit |

### Log viewer

| Key | Action |
|-----|--------|
| `j` / `k` or `↓` / `↑` | scroll |
| `Ctrl+d` / `Ctrl+u` | scroll by 10 lines |
| `g` / `G` | jump to top / bottom |
| `l` or `space` | toggle live preview |
| `/` | search in the log (regex, case-insensitive) |
| `n` / `N` | next / previous match |
| `s` | save the current buffer to `/tmp` |
| `q` or `Esc` | back to the picker |

## Configuration

Create `~/.config/lview/config.toml`:

```toml
ssh_command = "ssh"
max_log_lines = 10000
discovery_timeout = 10
extra_paths = ["/custom/logs"]
```

## License

MIT
