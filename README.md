# ifconfig-rs

[![CI](https://github.com/cumulus13/ifconfig-rs/actions/workflows/release.yml/badge.svg)](https://github.com/cumulus13/ifconfig-rs/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/ifconfig-rs.svg)](https://crates.io/crates/ifconfig-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> Advanced Windows network interface configuration tool with rich terminal output, backup/restore, and clipboard integration.

A modern Rust reimplementation of the classic `ifconfig` utility for Windows, featuring colored terminal output, configuration persistence, and zero dependency on WMI or PowerShell.

![Demo](https://raw.githubusercontent.com/cumulus13/ifconfig-rs/main/assets/demo.png)

## Features

- **Rich Terminal Output** — Colored tables and lists with full hex-RGB customization via config file
- **Interface Management** — View, configure, and manage network adapters
- **Static & DHCP** — Switch between static IP/DNS and DHCP effortlessly
- **Backup & Restore** — Automatic configuration snapshots before changes
- **Clipboard Integration** — Copy IP or gateway to clipboard with one flag
- **Zero WMI / Zero PowerShell** — Pure Win32 API + `netsh` for maximum compatibility
- **Customizable Themes** — JSON config for colors, styles, and backup paths

## Installation

### Pre-built Binary

Download the latest release from [GitHub Releases](https://github.com/cumulus13/ifconfig-rs/releases):

```powershell
# Using PowerShell
Invoke-WebRequest -Uri "https://github.com/cumulus13/ifconfig-rs/releases/latest/download/ifconfig-windows-x86_64.exe" -OutFile "ifconfig.exe"
```

### From crates.io

```bash
cargo install ifconfig-rs
```

### From Source

```bash
git clone https://github.com/cumulus13/ifconfig-rs.git
cd ifconfig-rs
cargo build --release
# Binary will be at target/release/ifconfig.exe
```

## Usage

### Display all interfaces (list view)

```bash
ifconfig
```

### Display all interfaces (table view)

```bash
ifconfig -t
```

### Display specific interface

```bash
ifconfig -i "Wi-Fi"
ifconfig -i eth0 -t
```

### Copy IP to clipboard

```bash
ifconfig -g vmnet8
```

### Copy gateway to clipboard

```bash
ifconfig -G vmnet8
```

### Set static IP, netmask, gateway, and DNS

```bash
ifconfig -i vmnet8 -s 192.168.44.1 255.255.255.0 192.168.44.1 8.8.8.8 1.1.1.1
```

### Set DNS only

```bash
ifconfig -i vmnet8 -d 8.8.8.8 1.1.1.1
```

### Reset to DHCP

```bash
ifconfig -i vmnet8 --dhcp
```

### Restore previous configuration

```bash
ifconfig -i vmnet8 --restore
```

### Show usage examples

```bash
ifconfig --examples
```

## Configuration

On first run, a default config is created at `%USERPROFILE%\.ifconfig.json`:

```json
{
  "colors": {
    "interface_name": {
      "fg": "#FFFFFF",
      "bg": "#0000FF",
      "attributes": ["bold", "italic"]
    },
    "label_ipv4": {
      "fg": "#FFAA00",
      "attributes": ["bold"]
    },
    "value_ipv4": {
      "fg": "#FFFF00"
    },
    "success": {
      "fg": "#00FF00"
    },
    "error": {
      "fg": "#FF0000"
    }
    // ... (see full schema in source)
  },
  "backup_file": "~/.ifconfig_backup.json"
}
```

Place a `config.json` next to the executable to override the user-level config.

### Color Attributes

| Attribute | Effect |
|-----------|--------|
| `bold` | Bold text |
| `dim` | Dimmed text |
| `italic` | Italic text |
| `underline` | Underlined text |

## Backup File

Configuration backups are stored as JSON at `~/.ifconfig_backup.json` (or your custom path). Each interface's last state is keyed by name:

```json
{
  "vmnet8": {
    "ipv4": "192.168.44.1",
    "netmask": "255.255.255.0",
    "gateway": "192.168.44.1",
    "dns": ["8.8.8.8", "1.1.1.1"]
  }
}
```

## Requirements

- Windows 10/11 or Windows Server 2016+
- Administrator privileges for `-s`, `-d`, `--dhcp`, and `--restore` operations

## Architecture

| Component | Implementation |
|-----------|---------------|
| Adapter Enumeration | `GetAdaptersAddresses` (Win32 API) |
| DNS Resolution | Windows Registry (`Tcpip\Parameters\Interfaces\{GUID}`) |
| IP Configuration | `netsh.exe` (native Windows tool) |
| Clipboard | `arboard` crate |
| Colors | ANSI 24-bit RGB escape sequences |
| Config | JSON with serde |

## Building

```bash
# Debug build
cargo build

# Optimized release build
cargo build --release

# Run tests
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)