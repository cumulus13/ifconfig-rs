//! File: src\main.rs
//! Author: Hadi Cahyadi <cumulus13@gmail.com>
//! Date: 2026-05-05
//! Description: 
//! License: MIT

//! Advanced Windows network interface configuration tool.
//!
//! A Rust reimplementation of the classic `ifconfig` utility,
//! enhanced with rich terminal output, configuration backup/restore,
//! and full Windows network adapter management via native APIs.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use arboard::Clipboard;
use clap::Parser;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_INCLUDE_PREFIX,
    IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Networking::WinSock::{AF_INET, AF_UNSPEC, SOCKADDR_IN};
use windows::Win32::UI::Shell::IsUserAnAdmin;

// =============================================================================
// ERROR HANDLING
// =============================================================================

#[derive(Error, Debug)]
enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Windows API error: {0}")]
    WindowsApi(u32),

    #[error("Interface '{0}' not found")]
    InterfaceNotFound(String),

    #[error("No backup configuration found for '{0}'")]
    NoBackup(String),

    #[error("Administrator privileges required")]
    NotAdmin,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, AppError>;

// =============================================================================
// CONFIGURATION & STYLING
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct StyleDef {
    fg: Option<String>,
    bg: Option<String>,
    #[serde(default)]
    attributes: Vec<String>,
}

impl Default for StyleDef {
    fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            attributes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Colors {
    interface_name: StyleDef,
    label_status: StyleDef,
    value_status_up: StyleDef,
    value_status_down: StyleDef,
    label_ipv4: StyleDef,
    value_ipv4: StyleDef,
    label_netmask: StyleDef,
    value_netmask: StyleDef,
    label_mac: StyleDef,
    value_mac: StyleDef,
    label_gateway: StyleDef,
    value_gateway: StyleDef,
    label_dns: StyleDef,
    value_dns: StyleDef,
    value_speed: StyleDef,
    value_mtu: StyleDef,
    table_header: StyleDef,
    table_interface: StyleDef,
    table_ipv4: StyleDef,
    table_mac: StyleDef,
    table_gateway: StyleDef,
    table_dns: StyleDef,
    error: StyleDef,
    success: StyleDef,
    warning: StyleDef,
    info_cyan: StyleDef,
    dim: StyleDef,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            interface_name: StyleDef {
                fg: Some("#FFFFFF".into()),
                bg: Some("#0000FF".into()),
                attributes: vec!["bold".into(), "italic".into()],
            },
            label_status: StyleDef {
                fg: Some("#FFFF00".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            value_status_up: StyleDef::default(),
            value_status_down: StyleDef::default(),
            label_ipv4: StyleDef {
                fg: Some("#FFAA00".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            value_ipv4: StyleDef {
                fg: Some("#FFFF00".into()),
                ..Default::default()
            },
            label_netmask: StyleDef {
                fg: Some("#00FF00".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            value_netmask: StyleDef {
                fg: Some("#0055FF".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            label_mac: StyleDef {
                fg: Some("#FFFF00".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            value_mac: StyleDef {
                fg: Some("#00FF00".into()),
                ..Default::default()
            },
            label_gateway: StyleDef {
                fg: Some("#0000FF".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            value_gateway: StyleDef {
                fg: Some("#00FFFF".into()),
                ..Default::default()
            },
            label_dns: StyleDef {
                fg: Some("#FF00FF".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            value_dns: StyleDef::default(),
            value_speed: StyleDef {
                fg: Some("#00FF00".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            value_mtu: StyleDef {
                fg: Some("#0000FF".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            table_header: StyleDef {
                fg: Some("#0000FF".into()),
                attributes: vec!["bold".into()],
                ..Default::default()
            },
            table_interface: StyleDef {
                fg: Some("#00FFFF".into()),
                ..Default::default()
            },
            table_ipv4: StyleDef {
                fg: Some("#00FF00".into()),
                ..Default::default()
            },
            table_mac: StyleDef {
                fg: Some("#FFFF00".into()),
                ..Default::default()
            },
            table_gateway: StyleDef {
                fg: Some("#0000FF".into()),
                ..Default::default()
            },
            table_dns: StyleDef {
                fg: Some("#FF00FF".into()),
                ..Default::default()
            },
            error: StyleDef {
                fg: Some("#FF0000".into()),
                ..Default::default()
            },
            success: StyleDef {
                fg: Some("#00FF00".into()),
                ..Default::default()
            },
            warning: StyleDef {
                fg: Some("#FFFF00".into()),
                ..Default::default()
            },
            info_cyan: StyleDef {
                fg: Some("#00FFFF".into()),
                ..Default::default()
            },
            dim: StyleDef {
                attributes: vec!["dim".into()],
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    colors: Colors,
    backup_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            colors: Colors::default(),
            backup_file: "~/.ifconfig_backup.json".into(),
        }
    }
}

// =============================================================================
// ANSI STYLING ENGINE
// =============================================================================

fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some((r, g, b))
}

fn apply_style(text: &str, style: &StyleDef) -> String {
    let mut codes = Vec::new();

    if let Some(fg) = &style.fg {
        if let Some((r, g, b)) = parse_hex(fg) {
            codes.push(format!("38;2;{r};{g};{b}"));
        }
    }
    if let Some(bg) = &style.bg {
        if let Some((r, g, b)) = parse_hex(bg) {
            codes.push(format!("48;2;{r};{g};{b}"));
        }
    }
    for attr in &style.attributes {
        match attr.as_str() {
            "bold" => codes.push("1".into()),
            "dim" => codes.push("2".into()),
            "italic" => codes.push("3".into()),
            "underline" => codes.push("4".into()),
            _ => {}
        }
    }

    if codes.is_empty() {
        text.to_string()
    } else {
        format!("\x1b[{}m{text}\x1b[0m", codes.join(";"))
    }
}

// =============================================================================
// CLI
// =============================================================================

#[derive(Parser)]
#[command(
    name = "ifconfig",
    version,
    about = "Advanced Windows network interface configuration tool",
    long_about = "A rich-featured Windows alternative to ifconfig/ipconfig \
                  with colored output, backup/restore, and clipboard integration."
)]
struct Args {
    /// Show output in rich table format
    #[arg(short = 't', long = "table")]
    table: bool,

    /// Specify interface name (supports partial matching)
    #[arg(short = 'i', long = "interface", value_name = "IFACE")]
    interface: Option<String>,

    /// Copy IPv4 address to clipboard
    #[arg(short = 'g', long = "get-ip", value_name = "IFACE")]
    get_ip: Option<String>,

    /// Copy gateway to clipboard
    #[arg(short = 'G', long = "get-gateway", value_name = "IFACE")]
    get_gateway: Option<String>,

    /// Set IP configuration: IP NETMASK [GATEWAY] [DNS1] [DNS2] ...
    #[arg(short = 's', long = "set", num_args = 2.., value_name = "CONFIG")]
    set: Option<Vec<String>>,

    /// Set DNS servers only: DNS1 DNS2 ...
    #[arg(short = 'd', long = "dns", num_args = 1.., value_name = "DNS")]
    dns: Option<Vec<String>>,

    /// Reset interface to DHCP
    #[arg(long = "dhcp")]
    dhcp: bool,

    /// Restore saved configuration from backup
    #[arg(long = "restore")]
    restore: bool,

    /// Show usage examples
    #[arg(long = "examples")]
    examples: bool,
}

// =============================================================================
// NETWORK INTERFACE MODEL
// =============================================================================

#[derive(Debug, Clone)]
struct Interface {
    name: String,
    status: String,
    speed: String,
    mtu: String,
    ipv4: String,
    netmask: String,
    mac: String,
    gateway: String,
    dns: Vec<String>,
}

// =============================================================================
// REGISTRY DNS READER (No WMI)
// =============================================================================

fn get_dns_from_registry(guid: &str) -> Vec<String> {
    let path = format!(
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\{guid}"
    );
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let Ok(key) = hklm.open_subkey(&path) else {
        return Vec::new();
    };

    // Static DNS takes precedence
    if let Ok(val) = key.get_value::<String, _>("NameServer") {
        let servers: Vec<String> = val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !servers.is_empty() {
            return servers;
        }
    }

    // Fallback to DHCP-assigned DNS
    if let Ok(val) = key.get_value::<String, _>("DhcpNameServer") {
        let servers: Vec<String> = val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !servers.is_empty() {
            return servers;
        }
    }

    Vec::new()
}

// =============================================================================
// WINDOWS API: ENUMERATE ADAPTERS
// =============================================================================

fn prefix_to_netmask(prefix: u8) -> String {
    if prefix == 0 || prefix > 32 {
        return "-".into();
    }
    let mask = u32::MAX.wrapping_shl(32 - prefix as u32);
    let b = mask.to_be_bytes();
    format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
}

unsafe fn get_interfaces() -> Result<Vec<Interface>> {
    let mut buf_len = 0u32;

    let mut ret = GetAdaptersAddresses(
        AF_UNSPEC.0 as u32,
        GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS,
        None,
        None,
        &mut buf_len,
    );

    if ret != ERROR_SUCCESS.0 && ret != ERROR_BUFFER_OVERFLOW.0 {
        return Err(AppError::WindowsApi(ret));
    }

    let mut buffer = vec![0u8; buf_len as usize];
    let adapter_ptr = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;

    ret = GetAdaptersAddresses(
        AF_UNSPEC.0 as u32,
        GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS,
        None,
        Some(adapter_ptr),
        &mut buf_len,
    );

    if ret != ERROR_SUCCESS.0 {
        return Err(AppError::WindowsApi(ret));
    }

    let mut interfaces = Vec::new();
    let mut cur = adapter_ptr;

    while !cur.is_null() {
        let a = &*cur;

        // FriendlyName (UTF-16)
        let name = if !a.FriendlyName.0.is_null() {
            let len = (0..)
                .take_while(|&i| *a.FriendlyName.0.add(i) != 0)
                .count();
            let slice = std::slice::from_raw_parts(a.FriendlyName.0, len);
            String::from_utf16_lossy(slice)
        } else {
            "Unknown".into()
        };

        // Adapter GUID (ASCII)
        let guid = if !a.AdapterName.0.is_null() {
            let len = (0..)
                .take_while(|&i| *a.AdapterName.0.add(i) != 0)
                .count();
            let slice = std::slice::from_raw_parts(a.AdapterName.0, len);
            String::from_utf8_lossy(slice).into_owned()
        } else {
            String::new()
        };

        let status = if a.OperStatus == IfOperStatusUp {
            "🟢 Up".into()
        } else {
            "🔴 Down".into()
        };

        let speed = if a.TransmitLinkSpeed > 0 {
            format!("{} Mbps", a.TransmitLinkSpeed / 1_000_000)
        } else {
            "-".into()
        };

        let mtu = a.Mtu.to_string();

        let mac = if a.PhysicalAddressLength > 0 {
            std::slice::from_raw_parts(a.PhysicalAddress.as_ptr(), a.PhysicalAddressLength as usize)
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(":")
        } else {
            "-".into()
        };

        // IPv4 + prefix
        let mut ipv4 = "-".to_string();
        let mut netmask = "-".to_string();
        let mut u = a.FirstUnicastAddress;

        while !u.is_null() {
            let ua = &*u;
            if let Some(sa) = ua.Address.lpSockaddr.as_ref() {
                if sa.sa_family == AF_INET {
                    let sin = *(ua.Address.lpSockaddr as *const SOCKADDR_IN);
                    let b = [
                        sin.sin_addr.S_un.S_un_b.s_b1,
                        sin.sin_addr.S_un.S_un_b.s_b2,
                        sin.sin_addr.S_un.S_un_b.s_b3,
                        sin.sin_addr.S_un.S_un_b.s_b4,
                    ];
                    ipv4 = format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
                    netmask = prefix_to_netmask(ua.OnLinkPrefixLength);
                    break;
                }
            }
            u = ua.Next;
        }

        // Gateway
        let mut gateway = "-".to_string();
        let mut g = a.FirstGatewayAddress;

        while !g.is_null() {
            let ga = &*g;
            if let Some(sa) = ga.Address.lpSockaddr.as_ref() {
                if sa.sa_family == AF_INET {
                    let sin = *(ga.Address.lpSockaddr as *const SOCKADDR_IN);
                    let b = [
                        sin.sin_addr.S_un.S_un_b.s_b1,
                        sin.sin_addr.S_un.S_un_b.s_b2,
                        sin.sin_addr.S_un.S_un_b.s_b3,
                        sin.sin_addr.S_un.S_un_b.s_b4,
                    ];
                    gateway = format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
                    break;
                }
            }
            g = ga.Next;
        }

        // DNS: registry first, API fallback
        let mut dns = get_dns_from_registry(&guid);
        if dns.is_empty() {
            let mut d = a.FirstDnsServerAddress;
            while !d.is_null() {
                let da = &*d;
                if let Some(sa) = da.Address.lpSockaddr.as_ref() {
                    if sa.sa_family == AF_INET {
                        let sin = *(da.Address.lpSockaddr as *const SOCKADDR_IN);
                        let b = [
                            sin.sin_addr.S_un.S_un_b.s_b1,
                            sin.sin_addr.S_un.S_un_b.s_b2,
                            sin.sin_addr.S_un.S_un_b.s_b3,
                            sin.sin_addr.S_un.S_un_b.s_b4,
                        ];
                        dns.push(format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]));
                    }
                }
                d = da.Next;
            }
        }

        interfaces.push(Interface {
            name,
            status,
            speed,
            mtu,
            ipv4,
            netmask,
            mac,
            gateway,
            dns,
        });

        cur = a.Next;
    }

    Ok(interfaces)
}

// =============================================================================
// CONFIG & BACKUP PERSISTENCE
// =============================================================================

fn config_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("config.json")));

    if let Some(ref p) = exe_dir {
        if p.exists() {
            return p.clone();
        }
    }

    let home_path = dirs::home_dir()
        .map(|d| d.join(".ifconfig.json"))
        .unwrap_or_else(|| PathBuf::from(".ifconfig.json"));

    if home_path.exists() {
        return home_path;
    }

    // Write default config
    if let Ok(json) = serde_json::to_string_pretty(&Config::default()) {
        let _ = fs::write(&home_path, json);
    }

    home_path
}

fn load_config() -> (Config, PathBuf) {
    let path = config_path();
    let cfg: Config = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let backup = if cfg.backup_file.starts_with("~/") || cfg.backup_file.starts_with("~\\") {
        dirs::home_dir()
            .map(|p| p.join(&cfg.backup_file[2..]))
            .unwrap_or_else(|| PathBuf::from(&cfg.backup_file))
    } else {
        PathBuf::from(&cfg.backup_file)
    };

    (cfg, backup)
}

fn save_current_config(iface_name: &str, interfaces: &[Interface], backup: &Path, cfg: &Config) -> Result<bool> {
    let Some(current) = interfaces
        .iter()
        .find(|i| i.name.to_lowercase().contains(&iface_name.to_lowercase()))
    else {
        println!(
            "{}",
            apply_style(&format!("Interface '{iface_name}' not found!"), &cfg.colors.error)
        );
        return Ok(false);
    };

    let mut map: serde_json::Map<String, serde_json::Value> = if backup.exists() {
        fs::read_to_string(backup)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        serde_json::Map::new()
    };

    map.insert(
        iface_name.into(),
        serde_json::json!({
            "ipv4": current.ipv4,
            "netmask": current.netmask,
            "gateway": current.gateway,
            "dns": current.dns,
        }),
    );

    fs::write(backup, serde_json::to_string_pretty(&map)?)?;
    println!(
        "{}",
        apply_style(&format!("✓ Configuration saved for {iface_name}"), &cfg.colors.success)
    );
    Ok(true)
}

// =============================================================================
// NETSH COMMAND WRAPPERS (No PowerShell)
// =============================================================================

fn netsh_error(msg: &str, stderr: &[u8], stdout: &[u8], cfg: &Config) {
    let text = String::from_utf8_lossy(stderr);
    let text = if text.trim().is_empty() {
        String::from_utf8_lossy(stdout)
    } else {
        text
    };
    let text = if text.trim().is_empty() {
        "Command failed with non-zero exit code".into()
    } else {
        text.into_owned()
    };

    println!(
        "{}",
        apply_style(&format!("{msg}: {text}"), &cfg.colors.error)
    );
}

fn set_interface_config(
    iface: &str,
    ip: &str,
    netmask: &str,
    gateway: &str,
    dns: &[String],
    backup: &Path,
    cfg: &Config,
) -> Result<bool> {
    let interfaces = unsafe { get_interfaces()? };
    let _ = save_current_config(iface, &interfaces, backup, cfg)?;

    println!(
        "{}",
        apply_style(&format!("🔧 Configuring interface '{iface}'..."), &cfg.colors.warning)
    );

    // IP + netmask + optional gateway
    if ip != "0" && netmask != "0" {
        let has_gw = gateway != "0" && gateway != "-" && !gateway.is_empty();

        println!(
            "{}",
            apply_style(&format!("Setting IP: {ip}/{netmask}"), &cfg.colors.info_cyan)
        );
        if has_gw {
            println!(
                "{}",
                apply_style(&format!("Setting Gateway: {gateway}"), &cfg.colors.info_cyan)
            );
        }

        let mut cmd = Command::new("netsh");
        cmd.arg("interface")
            .arg("ip")
            .arg("set")
            .arg("address")
            .arg(iface)
            .arg("static")
            .arg(ip)
            .arg(netmask);
        if has_gw {
            cmd.arg(gateway);
        }

        let cmd_str = format!("{:?}", cmd);
        println!("{}", apply_style(&format!("Command: {cmd_str}"), &cfg.colors.dim));

        let out = cmd.output()?;
        if !out.status.success() {
            netsh_error("Error setting IP", &out.stderr, &out.stdout, cfg);
            return Ok(false);
        }
        println!(
            "{}",
            apply_style("✓ IP address set successfully", &cfg.colors.success)
        );
    }

    // DNS servers
    if !dns.is_empty() {
        let clear_cmd = format!(r#"netsh interface ip set dns "{iface}" dhcp"#);
        println!("{}", apply_style(&format!("Clearing DNS: {clear_cmd}"), &cfg.colors.dim));

        let _ = Command::new("netsh")
            .args(["interface", "ip", "set", "dns", iface, "dhcp"])
            .output();

        for (i, server) in dns.iter().enumerate() {
            if server == "0" || server == "-" || server.is_empty() {
                continue;
            }

            println!(
                "{}",
                apply_style(&format!("Setting DNS {}: {server}", i + 1), &cfg.colors.info_cyan)
            );

            let mut cmd = Command::new("netsh");
            if i == 0 {
                cmd.args(["interface", "ip", "set", "dns", iface, "static", server]);
            } else {
                cmd.args([
                    "interface",
                    "ip",
                    "add",
                    "dns",
                    iface,
                    server,
                    &format!("index={}", i + 1),
                ]);
            }

            let cmd_str = format!("{:?}", cmd);
            println!("{}", apply_style(&format!("Command: {cmd_str}"), &cfg.colors.dim));

            let out = cmd.output()?;
            if !out.status.success() {
                netsh_error("Error setting DNS", &out.stderr, &out.stdout, cfg);
            } else {
                println!(
                    "{}",
                    apply_style(&format!("✓ DNS {} set successfully", i + 1), &cfg.colors.success)
                );
            }
        }
    }

    println!(
        "{}",
        apply_style(&format!("✓ Interface '{iface}' configured successfully!"), &cfg.colors.success)
    );
    Ok(true)
}

fn set_dns_only(iface: &str, dns: &[String], cfg: &Config) -> Result<bool> {
    println!(
        "{}",
        apply_style(&format!("🔧 Setting DNS for interface '{iface}'..."), &cfg.colors.warning)
    );

    let clear_cmd = format!(r#"netsh interface ip set dns "{iface}" dhcp"#);
    println!("{}", apply_style(&format!("Clearing DNS: {clear_cmd}"), &cfg.colors.dim));

    let _ = Command::new("netsh")
        .args(["interface", "ip", "set", "dns", iface, "dhcp"])
        .output();

    for (i, server) in dns.iter().enumerate() {
        if server == "0" || server.is_empty() {
            continue;
        }

        println!(
            "{}",
            apply_style(&format!("Setting DNS {}: {server}", i + 1), &cfg.colors.info_cyan)
        );

        let mut cmd = Command::new("netsh");
        if i == 0 {
            cmd.args(["interface", "ip", "set", "dns", iface, "static", server]);
        } else {
            cmd.args([
                "interface",
                "ip",
                "add",
                "dns",
                iface,
                server,
                &format!("index={}", i + 1),
            ]);
        }

        let cmd_str = format!("{:?}", cmd);
        println!("{}", apply_style(&format!("Command: {cmd_str}"), &cfg.colors.dim));

        let out = cmd.output()?;
        if !out.status.success() {
            netsh_error("Error setting DNS", &out.stderr, &out.stdout, cfg);
            return Ok(false);
        }

        println!(
            "{}",
            apply_style(&format!("✓ DNS {} set successfully", i + 1), &cfg.colors.success)
        );
    }

    println!(
        "{}",
        apply_style(&format!("✓ DNS configured successfully for '{iface}'!"), &cfg.colors.success)
    );
    Ok(true)
}

fn set_dhcp(iface: &str, backup: &Path, cfg: &Config) -> Result<bool> {
    let interfaces = unsafe { get_interfaces()? };
    let _ = save_current_config(iface, &interfaces, backup, cfg)?;

    println!(
        "{}",
        apply_style(&format!("🔧 Resetting interface '{iface}' to DHCP..."), &cfg.colors.warning)
    );

    let out = Command::new("netsh")
        .args(["interface", "ip", "set", "address", iface, "dhcp"])
        .output()?;

    if !out.status.success() {
        netsh_error("Error setting DHCP", &out.stderr, &out.stdout, cfg);
        return Ok(false);
    }

    let out = Command::new("netsh")
        .args(["interface", "ip", "set", "dns", iface, "dhcp"])
        .output()?;

    if !out.status.success() {
        netsh_error("Error setting DNS to DHCP", &out.stderr, &out.stdout, cfg);
        return Ok(false);
    }

    println!(
        "{}",
        apply_style(&format!("✓ Interface '{iface}' reset to DHCP successfully!"), &cfg.colors.success)
    );
    Ok(true)
}

fn restore_config(iface: &str, backup: &Path, cfg: &Config) -> Result<bool> {
    if !backup.exists() {
        println!("{}", apply_style("No saved configuration found!", &cfg.colors.error));
        return Ok(false);
    }

    let content = fs::read_to_string(backup)?;
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&content)?;

    let Some(entry) = map.get(iface) else {
        println!(
            "{}",
            apply_style(&format!("No saved configuration for '{iface}'!"), &cfg.colors.error)
        );
        return Ok(false);
    };

    let ip = entry.get("ipv4").and_then(|v| v.as_str()).unwrap_or("0");
    let netmask = entry.get("netmask").and_then(|v| v.as_str()).unwrap_or("0");
    let gateway = entry.get("gateway").and_then(|v| v.as_str()).unwrap_or("0");
    let dns: Vec<String> = entry
        .get("dns")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    set_interface_config(iface, ip, netmask, gateway, &dns, backup, cfg)?;
    Ok(true)
}

// =============================================================================
// DISPLAY RENDERERS
// =============================================================================

fn print_as_table(interfaces: &[Interface], cfg: &Config) {
    println!("{}", apply_style("Network Interfaces", &cfg.colors.table_header));

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        apply_style("Interface", &cfg.colors.table_header),
        apply_style("Status", &cfg.colors.table_header),
        apply_style("Speed", &cfg.colors.table_header),
        apply_style("MTU", &cfg.colors.table_header),
        apply_style("IPv4", &cfg.colors.table_header),
        apply_style("Netmask", &cfg.colors.table_header),
        apply_style("MAC", &cfg.colors.table_header),
        apply_style("Gateway", &cfg.colors.table_header),
        apply_style("DNS Servers", &cfg.colors.table_header),
    ]);

    for iface in interfaces {
        let dns_str = if iface.dns.is_empty() {
            "-".into()
        } else {
            iface.dns.join(", ")
        };

        table.add_row(vec![
            apply_style(&iface.name, &cfg.colors.table_interface),
            iface.status.clone(),
            iface.speed.clone(),
            iface.mtu.clone(),
            apply_style(&iface.ipv4, &cfg.colors.table_ipv4),
            iface.netmask.clone(),
            apply_style(&iface.mac, &cfg.colors.table_mac),
            apply_style(&iface.gateway, &cfg.colors.table_gateway),
            apply_style(&dns_str, &cfg.colors.table_dns),
        ]);
    }

    println!("{table}");
}

fn print_as_list(interfaces: &[Interface], cfg: &Config) {
    for iface in interfaces {
        println!("{}", apply_style(&iface.name, &cfg.colors.interface_name));

        let status_lbl = apply_style("Status :", &cfg.colors.label_status);
        println!("  📶 {status_lbl} {}\n", iface.status);

        let ipv4_lbl = apply_style("IPv4   :", &cfg.colors.label_ipv4);
        let ipv4_val = apply_style(&iface.ipv4, &cfg.colors.value_ipv4);
        println!("  🌐 {ipv4_lbl} {ipv4_val}");

        let nm_lbl = apply_style("Netmask:", &cfg.colors.label_netmask);
        let nm_val = apply_style(&iface.netmask, &cfg.colors.value_netmask);
        println!("  🎯 {nm_lbl} {nm_val}");

        let mac_lbl = apply_style("MAC    :", &cfg.colors.label_mac);
        let mac_val = apply_style(&iface.mac, &cfg.colors.value_mac);
        println!("  🔗 {mac_lbl} {mac_val}");

        let gw_lbl = apply_style("Gateway:", &cfg.colors.label_gateway);
        let gw_val = apply_style(&iface.gateway, &cfg.colors.value_gateway);
        println!("  🚪 {gw_lbl} {gw_val}");

        let dns_lbl = apply_style("DNS    :", &cfg.colors.label_dns);
        let dns_val = if iface.dns.is_empty() {
            "-".into()
        } else {
            iface.dns.join(", ")
        };
        println!("  📡 {dns_lbl} {dns_val}\n");

        let spd_lbl = apply_style("Speed :", &cfg.colors.value_speed);
        println!("   ⚡️{spd_lbl} {}", iface.speed);

        let mtu_lbl = apply_style("MTU    :", &cfg.colors.value_mtu);
        println!("  🧱 {mtu_lbl} {}\n", iface.mtu);
    }
}

fn show_help_examples(cfg: &Config) {
    println!();
    println!("{}", apply_style("Usage Examples:", &cfg.colors.info_cyan));
    println!(
        "  {}",
        apply_style("# Set IP, netmask, gateway, DNS for vmnet8", &cfg.colors.success)
    );
    println!("  ifconfig.exe -i vmnet8 -s 192.168.44.1 255.255.255.0 192.168.44.1 8.8.8.8 1.1.1.1");
    println!("  {}", apply_style("# Set DNS only", &cfg.colors.success));
    println!("  ifconfig.exe -i vmnet8 -d 8.8.8.8 1.1.1.1");
    println!("  {}", apply_style("# Reset interface to DHCP", &cfg.colors.success));
    println!("  ifconfig.exe -i vmnet8 --dhcp");
    println!("  {}", apply_style("# Restore saved configuration", &cfg.colors.success));
    println!("  ifconfig.exe -i vmnet8 --restore");
    println!("  {}", apply_style("# Copy IP to clipboard", &cfg.colors.success));
    println!("  ifconfig.exe -g vmnet8");
}

// =============================================================================
// ADMIN CHECK
// =============================================================================

fn require_admin() -> Result<()> {
    unsafe {
        if !IsUserAnAdmin().as_bool() {
            return Err(AppError::NotAdmin);
        }
    }
    Ok(())
}

// =============================================================================
// MAIN
// =============================================================================

fn main() {
    // Initialize tracing for structured logging (no-op in release by default)
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let (cfg, backup_path) = load_config();

    if args.examples {
        show_help_examples(&cfg);
        return;
    }

    // Admin check for destructive operations
    if args.set.is_some() || args.dns.is_some() || args.dhcp || args.restore {
        if let Err(e) = require_admin() {
            println!(
                "{}",
                apply_style(&format!("⚠️  {e}"), &cfg.colors.error)
            );
            println!(
                "{}",
                apply_style("Please run as administrator or use 'Run as administrator'", &cfg.colors.warning)
            );
            return;
        }
    }

    let interfaces = unsafe {
        match get_interfaces() {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to enumerate network interfaces: {e}");
                println!(
                    "{}",
                    apply_style(&format!("Error reading interfaces: {e}"), &cfg.colors.error)
                );
                std::process::exit(1);
            }
        }
    };

    // Copy IP to clipboard
    if let Some(query) = args.get_ip {
        let found = interfaces
            .iter()
            .find(|i| i.name.to_lowercase().contains(&query.to_lowercase()));

        match found {
            Some(iface) => {
                match Clipboard::new() {
                    Ok(mut cb) => {
                        let _ = cb.set_text(iface.ipv4.clone());
                        println!("📋 Copied IP: {}", apply_style(&iface.ipv4, &cfg.colors.info_cyan));
                    }
                    Err(e) => {
                        println!("{}", apply_style(&format!("Clipboard error: {e}"), &cfg.colors.error));
                    }
                }
            }
            None => {
                println!(
                    "{}",
                    apply_style(&format!("Interface '{query}' not found!"), &cfg.colors.error)
                );
            }
        }
        return;
    }

    // Copy Gateway to clipboard
    if let Some(query) = args.get_gateway {
        let found = interfaces
            .iter()
            .find(|i| i.name.to_lowercase().contains(&query.to_lowercase()));

        match found {
            Some(iface) => {
                match Clipboard::new() {
                    Ok(mut cb) => {
                        let _ = cb.set_text(iface.gateway.clone());
                        println!("📋 Copied Gateway: {}", apply_style(&iface.gateway, &cfg.colors.info_cyan));
                    }
                    Err(e) => {
                        println!("{}", apply_style(&format!("Clipboard error: {e}"), &cfg.colors.error));
                    }
                }
            }
            None => {
                println!(
                    "{}",
                    apply_style(&format!("Interface '{query}' not found!"), &cfg.colors.error)
                );
            }
        }
        return;
    }

    // Interface-specific operations
    let mut display_interfaces = interfaces.clone();

    if let Some(query) = args.interface {
        let found = interfaces
            .iter()
            .find(|i| i.name.to_lowercase().contains(&query.to_lowercase()))
            .cloned();

        match found {
            Some(iface) => {
                let name = iface.name.clone();

                if args.restore {
                    if let Err(e) = restore_config(&name, &backup_path, &cfg) {
                        println!("{}", apply_style(&format!("Error: {e}"), &cfg.colors.error));
                    }
                    return;
                }
                if args.dhcp {
                    if let Err(e) = set_dhcp(&name, &backup_path, &cfg) {
                        println!("{}", apply_style(&format!("Error: {e}"), &cfg.colors.error));
                    }
                    return;
                }
                if let Some(values) = args.set {
                    if values.len() < 2 {
                        println!(
                            "{}",
                            apply_style("Error: -s requires at least IP and NETMASK", &cfg.colors.error)
                        );
                        return;
                    }
                    let ip = &values[0];
                    let netmask = &values[1];
                    let gateway = values.get(2).map(|s| s.as_str()).unwrap_or("0");
                    let dns: Vec<String> = values.iter().skip(3).cloned().collect();

                    if let Err(e) = set_interface_config(&name, ip, netmask, gateway, &dns, &backup_path, &cfg) {
                        println!("{}", apply_style(&format!("Error: {e}"), &cfg.colors.error));
                    }
                    return;
                }
                if let Some(dns) = args.dns {
                    if let Err(e) = set_dns_only(&name, &dns, &cfg) {
                        println!("{}", apply_style(&format!("Error: {e}"), &cfg.colors.error));
                    }
                    return;
                }

                // No operation: display filtered interface
                display_interfaces = vec![iface];
            }
            None => {
                println!(
                    "{}",
                    apply_style(&format!("Interface '{query}' not found!"), &cfg.colors.error)
                );
                return;
            }
        }
    }

    // Operations that require interface but none was given
    if args.set.is_some() || args.dns.is_some() || args.dhcp || args.restore {
        println!(
            "{}",
            apply_style("Error: Interface must be specified with -i/--interface", &cfg.colors.error)
        );
        return;
    }

    // Display output
    if args.table {
        print_as_table(&display_interfaces, &cfg);
    } else {
        print_as_list(&display_interfaces, &cfg);
    }
}