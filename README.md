# lview

TUI pro prohlížení logů na vzdálených Linux strojích přes SSH. Automaticky detekuje systémové logy, Docker kontejnery, `/opt` aplikace a systemd journal.

## Instalace

### macOS (Homebrew)

```bash
brew tap janvete/tools
brew install lview
```

### Debian / Ubuntu (.deb)

Stáhněte si nejnovější `.deb` z [GitHub Releases](https://github.com/janvete/lview/releases) a nainstalujte:

```bash
sudo apt install ./lview_*.deb
# nebo
sudo dpkg -i ./lview_*.deb
```

### Ze zdroje

```bash
cargo install --git https://github.com/janvete/lview
```

## Použití

```bash
lview ssh -p22 root@192.168.53.3
```

Všechny argumenty za `ssh` jsou předány přímo systémovému `ssh` příkazu, takže se použije vaše `~/.ssh/config`, SSH agent a klíče.

## Ovládání

### Výběr logu

| Klávesa | Akce |
|---------|------|
| `j` / `k` nebo `↓` / `↑` | pohyb v seznamu |
| `Enter` | otevřít vybraný log |
| `/` | fuzzy filtrování seznamu logů |
| `r` | znovu načíst seznam logů |
| `q` | ukončit |

### Prohlížeč logu

| Klávesa | Akce |
|---------|------|
| `j` / `k` nebo `↓` / `↑` | scroll |
| `Ctrl+d` / `Ctrl+u` | scroll o 10 řádků |
| `g` / `G` | začátek / konec |
| `l` nebo `mezerník` | zapnout/vypnout živý náhled |
| `/` | vyhledávání v logu (regex, case-insensitive) |
| `n` / `N` | další / předchozí výskyt |
| `s` | uložit aktuální buffer do `/tmp` |
| `q` nebo `Esc` | zpět do výběru |

## Konfigurace

Vytvořte `~/.config/lview/config.toml`:

```toml
ssh_command = "ssh"
max_log_lines = 10000
discovery_timeout = 10
extra_paths = ["/custom/logs"]
```

## License

MIT
