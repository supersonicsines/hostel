use std::collections::BTreeMap;
#[cfg(target_os = "linux")]
use std::collections::HashMap;

use anyhow::Result;

use crate::service::{detect_service_kind, is_service_port, LocalService, ServiceMetadata};

#[derive(Debug, Clone)]
#[cfg(target_os = "linux")]
struct PidInfo {
    pid: u32,
    process_name: String,
    command: String,
}

pub async fn scan_services() -> Result<Vec<LocalService>> {
    tokio::task::spawn_blocking(scan_services_sync)
        .await
        .map_err(Into::into)
}

fn scan_services_sync() -> Vec<LocalService> {
    #[cfg(target_os = "macos")]
    {
        scan_services_macos()
    }

    #[cfg(target_os = "linux")]
    {
        scan_services_linux()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn scan_services_macos() -> Vec<LocalService> {
    let output = std::process::Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_lsof_services(&stdout)
        }
        _ => Vec::new(),
    }
}

#[cfg(target_os = "linux")]
fn scan_services_linux() -> Vec<LocalService> {
    let inode_map = build_inode_pid_map();
    let mut services = Vec::new();

    if let Ok(content) = std::fs::read_to_string("/proc/net/tcp") {
        services.extend(parse_proc_net_services(
            &content,
            AddressFamily::Ipv4,
            &inode_map,
        ));
    }

    if let Ok(content) = std::fs::read_to_string("/proc/net/tcp6") {
        services.extend(parse_proc_net_services(
            &content,
            AddressFamily::Ipv6,
            &inode_map,
        ));
    }

    dedupe_and_sort(services)
}

fn parse_lsof_services(output: &str) -> Vec<LocalService> {
    let mut services = Vec::new();

    for line in output.lines().skip(1) {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            continue;
        }

        let process_name = parts[0].to_string();
        let Some(pid) = parts.get(1).and_then(|pid| pid.parse::<u32>().ok()) else {
            continue;
        };

        let Some(tcp_index) = parts.iter().position(|part| *part == "TCP") else {
            continue;
        };

        let Some(endpoint) = parts.get(tcp_index + 1) else {
            continue;
        };

        if let Some((address, port)) = parse_lsof_endpoint(endpoint) {
            let command = read_process_command(pid).unwrap_or_else(|| process_name.clone());
            services.push(LocalService {
                pid,
                port,
                address,
                process_name: process_name.clone(),
                command: command.clone(),
                kind: detect_service_kind(&process_name, &command),
                metadata: ServiceMetadata::default(),
            });
        }
    }

    dedupe_and_sort(services)
}

#[cfg(target_os = "macos")]
fn read_process_command(pid: u32) -> Option<String> {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "args="])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let command = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if command.is_empty() {
        None
    } else {
        Some(command)
    }
}

fn parse_lsof_endpoint(endpoint: &str) -> Option<(String, u16)> {
    let local = endpoint
        .split("->")
        .next()
        .unwrap_or(endpoint)
        .trim()
        .trim_end_matches("(LISTEN)");

    let (host, port) = if let Some(rest) = local.strip_prefix('[') {
        let (host, port) = rest.split_once("]:")?;
        (host, port.parse::<u16>().ok()?)
    } else {
        let (host, port) = local.rsplit_once(':')?;
        (host, port.parse::<u16>().ok()?)
    };

    if !is_service_port(port) || !is_loopback_host(host) {
        return None;
    }

    Some((normalize_host(host), port))
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy)]
enum AddressFamily {
    Ipv4,
    Ipv6,
}

#[cfg(target_os = "linux")]
fn build_inode_pid_map() -> HashMap<String, PidInfo> {
    let mut map = HashMap::new();

    let Ok(entries) = std::fs::read_dir("/proc") else {
        return map;
    };

    for entry in entries.flatten() {
        let pid = entry.file_name().to_string_lossy().parse::<u32>().ok();
        let Some(pid) = pid else {
            continue;
        };

        let process_name = read_process_name(pid);
        let command = read_process_command(pid).unwrap_or_else(|| process_name.clone());
        let info = PidInfo {
            pid,
            process_name,
            command,
        };

        let fd_dir = entry.path().join("fd");
        let Ok(fd_entries) = std::fs::read_dir(fd_dir) else {
            continue;
        };

        for fd in fd_entries.flatten() {
            let Ok(link) = std::fs::read_link(fd.path()) else {
                continue;
            };
            let link = link.to_string_lossy();
            let Some(inode) = link
                .strip_prefix("socket:[")
                .and_then(|value| value.strip_suffix(']'))
            else {
                continue;
            };
            map.entry(inode.to_string()).or_insert_with(|| info.clone());
        }
    }

    map
}

#[cfg(target_os = "linux")]
fn read_process_name(pid: u32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(target_os = "linux")]
fn read_process_command(pid: u32) -> Option<String> {
    let bytes = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let command = bytes
        .split(|byte| *byte == 0)
        .filter_map(|part| std::str::from_utf8(part).ok())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if command.is_empty() {
        None
    } else {
        Some(command)
    }
}

#[cfg(target_os = "linux")]
fn parse_proc_net_services(
    content: &str,
    family: AddressFamily,
    inode_map: &HashMap<String, PidInfo>,
) -> Vec<LocalService> {
    let mut services = Vec::new();

    for line in content.lines().skip(1) {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 10 || parts[3] != "0A" {
            continue;
        }

        let local = parts[1];
        let inode = parts[9];
        let Some(info) = inode_map.get(inode) else {
            continue;
        };
        let Some((address, port)) = parse_proc_endpoint(local, family) else {
            continue;
        };

        services.push(LocalService {
            pid: info.pid,
            port,
            address,
            process_name: info.process_name.clone(),
            command: info.command.clone(),
            kind: detect_service_kind(&info.process_name, &info.command),
            metadata: ServiceMetadata::default(),
        });
    }

    services
}

#[cfg(target_os = "linux")]
fn parse_proc_endpoint(endpoint: &str, family: AddressFamily) -> Option<(String, u16)> {
    let (address_hex, port_hex) = endpoint.split_once(':')?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    if !is_service_port(port) {
        return None;
    }

    let address = match family {
        AddressFamily::Ipv4 => decode_ipv4_loopback(address_hex)?,
        AddressFamily::Ipv6 => decode_ipv6_loopback(address_hex)?,
    };

    Some((address, port))
}

#[cfg(target_os = "linux")]
fn decode_ipv4_loopback(hex: &str) -> Option<String> {
    if hex.len() != 8 {
        return None;
    }

    let mut bytes = Vec::new();
    for idx in (0..hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hex[idx..idx + 2], 16).ok()?);
    }
    bytes.reverse();

    if bytes.first().copied() != Some(127) {
        return None;
    }

    Some(format!(
        "{}.{}.{}.{}",
        bytes[0], bytes[1], bytes[2], bytes[3]
    ))
}

#[cfg(target_os = "linux")]
fn decode_ipv6_loopback(hex: &str) -> Option<String> {
    match hex {
        "00000000000000000000000001000000" | "00000000000000000000000000000001" => {
            Some("::1".to_string())
        }
        _ => None,
    }
}

fn is_loopback_host(host: &str) -> bool {
    let host = host.trim_matches(['[', ']']);
    host == "localhost" || host == "::1" || host.starts_with("127.")
}

fn normalize_host(host: &str) -> String {
    let host = host.trim_matches(['[', ']']);
    if host == "localhost" {
        "127.0.0.1".to_string()
    } else {
        host.to_string()
    }
}

fn dedupe_and_sort(services: Vec<LocalService>) -> Vec<LocalService> {
    let mut by_port_pid = BTreeMap::new();
    for service in services {
        by_port_pid
            .entry((service.port, service.pid))
            .or_insert(service);
    }
    by_port_pid.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsof_parser_keeps_only_loopback_service_ports() {
        let output = "\
COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
node     1111 ama   22u  IPv4  12345      0t0  TCP 127.0.0.1:5173 (LISTEN)
python   2222 ama   10u  IPv4  12346      0t0  TCP *:8000 (LISTEN)
nginx    3333 ama   11u  IPv4  12347      0t0  TCP 127.0.0.1:80 (LISTEN)
astro    4444 ama   12u  IPv6  12348      0t0  TCP [::1]:4321 (LISTEN)
";

        let services = parse_lsof_services(output);
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].port, 4321);
        assert_eq!(services[1].port, 5173);
    }

    #[test]
    fn lsof_endpoint_rejects_public_and_wildcard_hosts() {
        assert!(parse_lsof_endpoint("0.0.0.0:3000").is_none());
        assert!(parse_lsof_endpoint("*:3000").is_none());
        assert!(parse_lsof_endpoint("192.168.1.9:3000").is_none());
        assert_eq!(
            parse_lsof_endpoint("localhost:3000"),
            Some(("127.0.0.1".to_string(), 3000))
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn proc_parser_keeps_loopback_listeners_in_range() {
        let mut inodes = HashMap::new();
        inodes.insert(
            "111".to_string(),
            PidInfo {
                pid: 900,
                process_name: "node".to_string(),
                command: "npm run dev".to_string(),
            },
        );
        inodes.insert(
            "222".to_string(),
            PidInfo {
                pid: 901,
                process_name: "python".to_string(),
                command: "uvicorn app:app".to_string(),
            },
        );

        let content = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:1435 00000000:0000 0A 00000000:00000000 00:00000000 00000000   501        0 111 1 0000000000000000 100 0 0 10 0
   1: 00000000:1F40 00000000:0000 0A 00000000:00000000 00:00000000 00000000   501        0 222 1 0000000000000000 100 0 0 10 0
";
        let services = parse_proc_net_services(content, AddressFamily::Ipv4, &inodes);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].address, "127.0.0.1");
        assert_eq!(services[0].port, 5173);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn proc_ipv6_loopback_is_decoded() {
        assert_eq!(
            parse_proc_endpoint("00000000000000000000000001000000:1A0B", AddressFamily::Ipv6),
            Some(("::1".to_string(), 6667))
        );
    }

    #[test]
    fn service_sorting_is_by_port_then_pid() {
        let services = dedupe_and_sort(vec![
            LocalService {
                pid: 3,
                port: 9000,
                address: "127.0.0.1".to_string(),
                process_name: "b".to_string(),
                command: "b".to_string(),
                kind: crate::service::ServiceKind::Unknown,
                metadata: ServiceMetadata::default(),
            },
            LocalService {
                pid: 2,
                port: 3000,
                address: "127.0.0.1".to_string(),
                process_name: "a".to_string(),
                command: "a".to_string(),
                kind: crate::service::ServiceKind::Unknown,
                metadata: ServiceMetadata::default(),
            },
        ]);

        assert_eq!(services[0].port, 3000);
        assert_eq!(services[1].port, 9000);
    }
}
