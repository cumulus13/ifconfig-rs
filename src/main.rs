// File: src\main.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Date: 2026-05-05
// Description: 
// License: MIT

use clap::Parser;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS,
    GAA_FLAG_INCLUDE_PREFIX, IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Networking::WinSock::{AF_INET, AF_UNSPEC, SOCKADDR_IN};
use windows::Win32::UI::Shell::IsUserAnAdmin;

/* ================================================================
   Config & Styles
   ================================================================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StyleDef {
    #[serde(default)]
    fg: Option<String>,
    #[serde(default)]
    bg: Option<String>,
    #[serde(default)]
    attributes: Vec<String>,
}

impl Default for StyleDef {
    fn default() -> Self {
        StyleDef {
            fg: None,
            bg: None,
            attributes: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    colors: Colors,
    backup_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            colors: Colors {
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
            },
            backup_file: "~/.ifconfig_backup.json".into(),
        }
    }
}

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
    let mut codes: Vec<String> = Vec::new();

    if let Some(fg) = &style.fg {
        if let Some((r, g, b)) = parse_hex(fg) {
            codes.push(format!("38;2;{};{};{}", r, g, b));
        }
    }
    if let Some(bg) = &style.bg {
        if let Some((r, g, b)) = parse_hex(bg) {
            codes.push(format!("48;2;{};{};{}", r, g, b));
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
        format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text)
    }
}

/* ================================================================
   CLI
   ================================================================ */

#[derive(Parser)]
#[command(name = "ifconfig")]
#[command(about = "Advanced ifconfig alternative with rich output and network configuration")]
struct Args {
    #[arg(short = 't', long = "table", help = "Show output in rich table format")]
    table: bool,

    #[arg(short = 'i', long = "interface", help = "Specify interface name (supports partial name)")]
    interface: Option<String>,

    #[arg(short = 'g', long = "get-ip", value_name = "IFACE", help = "Copy IPv4 address to clipboard")]
    get_ip: Option<String>,

    #[arg(short = 'G', long = "get-gateway", value_name = "IFACE", help = "Copy gateway to clipboard")]
    get_gateway: Option<String>,

    #[arg(
        short = 's',
        long = "set",
        num_args = 1..,
        value_name = "CONFIG",
        help = "Set IP configuration: IP NETMASK GATEWAY DNS1 DNS2 ..."
    )]
    set: Option<Vec<String>>,

    #[arg(
        short = 'd',
        long = "dns",
        num_args = 1..,
        value_name = "DNS",
        help = "Set DNS servers only: DNS1 DNS2 ..."
    )]
    dns: Option<Vec<String>>,

    #[arg(long = "dhcp", help = "Reset interface to DHCP")]
    dhcp: bool,

    #[arg(long = "restore", help = "Restore saved configuration")]
    restore: bool,

    #[arg(long = "examples", help = "Show usage examples")]
    examples: bool,
}

/* ================================================================
   Network Interface (GetAdaptersAddresses — no WMI)
   ================================================================ */

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

fn prefix_to_netmask(prefix: u8) -> String {
    if prefix > 32 {
        return "-".into();
    }
    let mask: u32 = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    let b = mask.to_be_bytes();
    format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
}

unsafe fn get_interfaces() -> Result<Vec<Interface>, io::Error> {
    let mut buf_len = 0u32;
    let mut ret = GetAdaptersAddresses(
        AF_UNSPEC.0 as u32,
        GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS,
        None,
        None,
        &mut buf_len,
    );

    if ret != ERROR_SUCCESS.0 && ret != ERROR_BUFFER_OVERFLOW.0 {
        return Err(io::Error::last_os_error());
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
        return Err(io::Error::last_os_error());
    }

    let mut out = Vec::new();
    let mut cur = adapter_ptr;

    while !cur.is_null() {
        let a = &*cur;

        // FriendlyName (wide string) — PWSTR.0 is the raw *mut u16
        let name = if !a.FriendlyName.0.is_null() {
            let len = (0..)
                .take_while(|&i| *a.FriendlyName.0.add(i) != 0)
                .count();
            let slice = std::slice::from_raw_parts(a.FriendlyName.0, len);
            String::from_utf16_lossy(slice)
        } else {
            "Unknown".into()
        };

        // Status
        let status = if a.OperStatus == IfOperStatusUp {
            "🟢 Up".into()
        } else {
            "🔴 Down".into()
        };

        // Speed (bits/s → Mbps)
        let speed = if a.TransmitLinkSpeed > 0 {
            format!("{} Mbps", a.TransmitLinkSpeed / 1_000_000)
        } else {
            "-".into()
        };

        let mtu = a.Mtu.to_string();

        // MAC
        let mac = if a.PhysicalAddressLength > 0 {
            let bytes = std::slice::from_raw_parts(
                a.PhysicalAddress.as_ptr(),
                a.PhysicalAddressLength as usize,
            );
            bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(":")
        } else {
            "-".into()
        };

        // IPv4 + netmask (first unicast IPv4)
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

        // Gateway (first IPv4 gateway)
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

        // DNS servers
        let mut dns = Vec::new();
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

        out.push(Interface {
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

    Ok(out)
}

/* ================================================================
   Config / Backup persistence
   ================================================================ */

fn config_path() -> PathBuf {
    let exe_cfg = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("config.json")));
    let home_cfg = dirs::home_dir().map(|d| d.join(".ifconfig.json"));

    if let Some(ref p) = exe_cfg {
        if p.exists() {
            return p.clone();
        }
    }
    if let Some(ref p) = home_cfg {
        if p.exists() {
            return p.clone();
        }
        // write default
        let default = Config::default();
        let _ = fs::write(p, serde_json::to_string_pretty(&default).unwrap_or_default());
        return p.clone();
    }
    PathBuf::from("config.json")
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

fn save_current_config(
    iface_name: &str,
    ifaces: &[Interface],
    backup: &Path,
    cfg: &Config,
) -> io::Result<bool> {
    let current = ifaces
        .iter()
        .find(|i| i.name.to_lowercase().contains(&iface_name.to_lowercase()))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Interface not found"))?;

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
        apply_style(&format!("✓ Configuration saved for {}", iface_name), &cfg.colors.success)
    );
    Ok(true)
}

/* ================================================================
   netsh wrappers (no PowerShell)
   ================================================================ */

fn set_interface_config(
    iface: &str,
    ip: &str,
    netmask: &str,
    gateway: &str,
    dns: &[String],
    backup: &Path,
    cfg: &Config,
) -> io::Result<bool> {
    let ifaces = unsafe { get_interfaces()? };
    let _ = save_current_config(iface, &ifaces, backup, cfg);

    println!(
        "{}",
        apply_style(
            &format!("🔧 Configuring interface '{}'...", iface),
            &cfg.colors.warning
        )
    );

    // IP + netmask + optional gateway
    if ip != "0" && netmask != "0" {
        let mut cmd = Command::new("netsh");
        cmd.arg("interface")
            .arg("ip")
            .arg("set")
            .arg("address")
            .arg(iface)
            .arg("static")
            .arg(ip)
            .arg(netmask);

        let has_gw = gateway != "0" && gateway != "-" && !gateway.is_empty();
        if has_gw {
            cmd.arg(gateway);
        }

        println!(
            "{}",
            apply_style(&format!("Setting IP: {}/{}", ip, netmask), &cfg.colors.info_cyan)
        );
        if has_gw {
            println!(
                "{}",
                apply_style(&format!("Setting Gateway: {}", gateway), &cfg.colors.info_cyan)
            );
        }

        let out = cmd.output()?;
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr);
            let msg = if msg.trim().is_empty() {
                String::from_utf8_lossy(&out.stdout).into_owned()
            } else {
                msg.into_owned()
            };
            println!(
                "{}",
                apply_style(&format!("Error setting IP: {}", msg.trim()), &cfg.colors.error)
            );
            return Ok(false);
        } else {
            println!(
                "{}",
                apply_style("✓ IP address set successfully", &cfg.colors.success)
            );
        }
    }

    // DNS
    if !dns.is_empty() {
        let _ = Command::new("netsh")
            .args(["interface", "ip", "set", "dns", iface, "dhcp"])
            .output();

        for (i, server) in dns.iter().enumerate() {
            if server == "0" || server == "-" || server.is_empty() {
                continue;
            }
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

            println!(
                "{}",
                apply_style(
                    &format!("Setting DNS {}: {}", i + 1, server),
                    &cfg.colors.info_cyan
                )
            );

            let out = cmd.output()?;
            if !out.status.success() {
                let msg = String::from_utf8_lossy(&out.stderr);
                let msg = if msg.trim().is_empty() {
                    String::from_utf8_lossy(&out.stdout).into_owned()
                } else {
                    msg.into_owned()
                };
                println!(
                    "{}",
                    apply_style(&format!("Error setting DNS: {}", msg.trim()), &cfg.colors.error)
                );
            } else {
                println!(
                    "{}",
                    apply_style(
                        &format!("✓ DNS {} set successfully", i + 1),
                        &cfg.colors.success
                    )
                );
            }
        }
    }

    println!(
        "{}",
        apply_style(
            &format!("✓ Interface '{}' configured successfully!", iface),
            &cfg.colors.success
        )
    );
    Ok(true)
}

fn set_dns_only(iface: &str, dns: &[String], cfg: &Config) -> io::Result<bool> {
    println!(
        "{}",
        apply_style(
            &format!("🔧 Setting DNS for interface '{}'...", iface),
            &cfg.colors.warning
        )
    );

    let _ = Command::new("netsh")
        .args(["interface", "ip", "set", "dns", iface, "dhcp"])
        .output();

    for (i, server) in dns.iter().enumerate() {
        if server == "0" || server.is_empty() {
            continue;
        }
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

        println!(
            "{}",
            apply_style(
                &format!("Setting DNS {}: {}", i + 1, server),
                &cfg.colors.info_cyan
            )
        );

        let out = cmd.output()?;
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr);
            let msg = if msg.trim().is_empty() {
                String::from_utf8_lossy(&out.stdout).into_owned()
            } else {
                msg.into_owned()
            };
            println!(
                "{}",
                apply_style(&format!("Error setting DNS: {}", msg.trim()), &cfg.colors.error)
            );
            return Ok(false);
        } else {
            println!(
                "{}",
                apply_style(
                    &format!("✓ DNS {} set successfully", i + 1),
                    &cfg.colors.success
                )
            );
        }
    }

    println!(
        "{}",
        apply_style(
            &format!("✓ DNS configured successfully for '{}'!", iface),
            &cfg.colors.success
        )
    );
    Ok(true)
}

fn set_dhcp(iface: &str, backup: &Path, cfg: &Config) -> io::Result<bool> {
    let ifaces = unsafe { get_interfaces()? };
    let _ = save_current_config(iface, &ifaces, backup, cfg);

    println!(
        "{}",
        apply_style(
            &format!("🔧 Resetting interface '{}' to DHCP...", iface),
            &cfg.colors.warning
        )
    );

    let out = Command::new("netsh")
        .args(["interface", "ip", "set", "address", iface, "dhcp"])
        .output()?;
    if !out.status.success() {
        println!(
            "{}",
            apply_style(
                &format!("Error setting DHCP: {}", String::from_utf8_lossy(&out.stderr).trim()),
                &cfg.colors.error
            )
        );
        return Ok(false);
    }

    let out = Command::new("netsh")
        .args(["interface", "ip", "set", "dns", iface, "dhcp"])
        .output()?;
    if !out.status.success() {
        println!(
            "{}",
            apply_style(
                &format!(
                    "Error setting DNS to DHCP: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
                &cfg.colors.error
            )
        );
        return Ok(false);
    }

    println!(
        "{}",
        apply_style(
            &format!("✓ Interface '{}' reset to DHCP successfully!", iface),
            &cfg.colors.success
        )
    );
    Ok(true)
}

fn restore_config(iface: &str, backup: &Path, cfg: &Config) -> io::Result<bool> {
    if !backup.exists() {
        println!(
            "{}",
            apply_style("No saved configuration found!", &cfg.colors.error)
        );
        return Ok(false);
    }

    let content = fs::read_to_string(backup)?;
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&content)?;

    let entry = map.get(iface).ok_or_else(|| {
        println!(
            "{}",
            apply_style(
                &format!("No saved configuration for '{}'!", iface),
                &cfg.colors.error
            )
        );
        io::Error::new(io::ErrorKind::NotFound, "missing backup")
    })?;

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

/* ================================================================
   Display
   ================================================================ */

fn print_as_table(ifaces: &[Interface], cfg: &Config) {
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

    for i in ifaces {
        table.add_row(vec![
            apply_style(&i.name, &cfg.colors.table_interface),
            i.status.clone(),
            i.speed.clone(),
            i.mtu.clone(),
            apply_style(&i.ipv4, &cfg.colors.table_ipv4),
            i.netmask.clone(),
            apply_style(&i.mac, &cfg.colors.table_mac),
            apply_style(&i.gateway, &cfg.colors.table_gateway),
            apply_style(&i.dns.join(", "), &cfg.colors.table_dns),
        ]);
    }

    println!("{}", table);
}

fn print_as_list(ifaces: &[Interface], cfg: &Config) {
    for i in ifaces {
        // Interface name
        println!("{}", apply_style(&i.name, &cfg.colors.interface_name));

        // Status (blank line after)
        let status_lbl = apply_style("Status :", &cfg.colors.label_status);
        println!("  📶 {} {}\n", status_lbl, i.status);

        // IPv4
        let ipv4_lbl = apply_style("IPv4   :", &cfg.colors.label_ipv4);
        let ipv4_val = apply_style(&i.ipv4, &cfg.colors.value_ipv4);
        println!("  🌐 {} {}", ipv4_lbl, ipv4_val);

        // Netmask
        let nm_lbl = apply_style("Netmask:", &cfg.colors.label_netmask);
        let nm_val = apply_style(&i.netmask, &cfg.colors.value_netmask);
        println!("  🎯 {} {}", nm_lbl, nm_val);

        // MAC
        let mac_lbl = apply_style("MAC    :", &cfg.colors.label_mac);
        let mac_val = apply_style(&i.mac, &cfg.colors.value_mac);
        println!("  🔗 {} {}", mac_lbl, mac_val);

        // Gateway
        let gw_lbl = apply_style("Gateway:", &cfg.colors.label_gateway);
        let gw_val = apply_style(&i.gateway, &cfg.colors.value_gateway);
        println!("  🚪 {} {}", gw_lbl, gw_val);

        // DNS (blank line after)
        let dns_lbl = apply_style("DNS    :", &cfg.colors.label_dns);
        let dns_val = if i.dns.is_empty() {
            "-".into()
        } else {
            i.dns.join(", ")
        };
        println!("  📡 {} {}\n", dns_lbl, dns_val);

        // Speed
        let spd_lbl = apply_style("Speed :", &cfg.colors.value_speed);
        println!("   ⚡️{} {}", spd_lbl, i.speed);

        // MTU (blank line after)
        let mtu_lbl = apply_style("MTU    :", &cfg.colors.value_mtu);
        println!("  🧱 {} {}\n", mtu_lbl, i.mtu);
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
    println!(
        "  {}",
        apply_style("# Set DNS only", &cfg.colors.success)
    );
    println!("  ifconfig.exe -i vmnet8 -d 8.8.8.8 1.1.1.1");
    println!(
        "  {}",
        apply_style("# Reset interface to DHCP", &cfg.colors.success)
    );
    println!("  ifconfig.exe -i vmnet8 --dhcp");
    println!(
        "  {}",
        apply_style("# Restore saved configuration", &cfg.colors.success)
    );
    println!("  ifconfig.exe -i vmnet8 --restore");
    println!(
        "  {}",
        apply_style("# Copy IP to clipboard", &cfg.colors.success)
    );
    println!("  ifconfig.exe -g vmnet8");
}

/* ================================================================
   Main
   ================================================================ */

fn main() {
    let args = Args::parse();
    let (cfg, backup_path) = load_config();

    if args.examples {
        show_help_examples(&cfg);
        return;
    }

    // Admin check for destructive ops
    if args.set.is_some() || args.dns.is_some() || args.dhcp || args.restore {
        unsafe {
            if !IsUserAnAdmin().as_bool() {
                println!(
                    "{}",
                    apply_style(
                        "⚠️  Administrator privileges required for network configuration!",
                        &cfg.colors.error
                    )
                );
                println!(
                    "{}",
                    apply_style(
                        "Please run as administrator or use 'Run as administrator'",
                        &cfg.colors.warning
                    )
                );
                return;
            }
        }
    }

    let interfaces = unsafe {
        match get_interfaces() {
            Ok(v) => v,
            Err(e) => {
                println!(
                    "{}",
                    apply_style(&format!("Error reading interfaces: {}", e), &cfg.colors.error)
                );
                return;
            }
        }
    };

    // Copy IP
    if let Some(query) = args.get_ip {
        let found = interfaces
            .iter()
            .find(|i| i.name.to_lowercase().contains(&query.to_lowercase()));
        if let Some(iface) = found {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(iface.ipv4.clone());
            }
            println!(
                "📋 Copied IP: {}",
                apply_style(&iface.ipv4, &cfg.colors.info_cyan)
            );
        } else {
            println!(
                "{}",
                apply_style(&format!("Interface '{}' not found!", query), &cfg.colors.error)
            );
        }
        return;
    }

    // Copy Gateway
    if let Some(query) = args.get_gateway {
        let found = interfaces
            .iter()
            .find(|i| i.name.to_lowercase().contains(&query.to_lowercase()));
        if let Some(iface) = found {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(iface.gateway.clone());
            }
            println!(
                "📋 Copied Gateway: {}",
                apply_style(&iface.gateway, &cfg.colors.info_cyan)
            );
        } else {
            println!(
                "{}",
                apply_style(&format!("Interface '{}' not found!", query), &cfg.colors.error)
            );
        }
        return;
    }

    // Interface-specific ops
    let mut display_ifaces = interfaces.clone();

    if let Some(query) = args.interface {
        let found = interfaces
            .iter()
            .find(|i| i.name.to_lowercase().contains(&query.to_lowercase()))
            .cloned();

        if let Some(iface) = found {
            let name = iface.name.clone();

            if args.restore {
                let _ = restore_config(&name, &backup_path, &cfg);
                return;
            }
            if args.dhcp {
                let _ = set_dhcp(&name, &backup_path, &cfg);
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
                let _ = set_interface_config(&name, ip, netmask, gateway, &dns, &backup_path, &cfg);
                return;
            }
            if let Some(dns) = args.dns {
                let _ = set_dns_only(&name, &dns, &cfg);
                return;
            }

            // No op: filter display to this interface
            display_ifaces = vec![iface];
        } else {
            println!(
                "{}",
                apply_style(&format!("Interface '{}' not found!", query), &cfg.colors.error)
            );
            return;
        }
    }

    // Ops requested but no interface given
    if args.set.is_some() || args.dns.is_some() || args.dhcp || args.restore {
        println!(
            "{}",
            apply_style(
                "Error: Interface must be specified with -i/--interface",
                &cfg.colors.error
            )
        );
        return;
    }

    // Display
    if args.table {
        print_as_table(&display_ifaces, &cfg);
    } else {
        print_as_list(&display_ifaces, &cfg);
    }
}