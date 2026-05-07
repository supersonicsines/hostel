use appcui::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const APP_NAME: &str = "localhostel";
const PRODUCT_VERSION: &str = "0.1000";
const LOCK_FILE_NAME: &str = "localhostel.lock";

#[derive(Debug)]
struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        release_lock(&self.path);
    }
}

fn lock_path() -> PathBuf {
    let base_dir = dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(std::env::temp_dir);

    base_dir.join(APP_NAME).join(LOCK_FILE_NAME)
}

fn acquire_lock() -> io::Result<LockGuard> {
    let path = lock_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    match write_lock_file(&path) {
        Ok(()) => Ok(LockGuard { path }),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists && is_stale_lock(&path) => {
            fs::remove_file(&path)?;
            write_lock_file(&path).map(|()| LockGuard { path })
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Err(io::Error::new(
            err.kind(),
            format!(
                "{} is already running{}",
                APP_NAME,
                existing_lock_hint(&path)
            ),
        )),
        Err(err) => Err(err),
    }
}

fn write_lock_file(path: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    writeln!(file, "{}", std::process::id())
}

fn release_lock(path: &Path) {
    if lock_pid(path) == Some(std::process::id()) {
        let _ = fs::remove_file(path);
    }
}

fn existing_lock_hint(path: &Path) -> String {
    lock_pid(path)
        .map(|pid| format!(" (pid {pid})"))
        .unwrap_or_default()
}

fn is_stale_lock(path: &Path) -> bool {
    match lock_pid(path) {
        Some(pid) if pid == std::process::id() => false,
        Some(pid) => !is_process_running(pid),
        None => true,
    }
}

fn lock_pid(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|pid| *pid > 0)
}

fn is_process_running(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }

    #[cfg(unix)]
    {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PortEntry {
    address: String,
    port: u16,
    pid: Option<u32>,
    process: String,
}

impl PortEntry {
    fn is_loopback(&self) -> bool {
        is_loopback_host(&self.address)
    }

    fn is_wildcard(&self) -> bool {
        is_wildcard_host(&self.address)
    }

    fn url(&self) -> String {
        format!("http://{}:{}", self.url_host(), self.port)
    }

    fn url_host(&self) -> String {
        if self.is_wildcard() {
            return "localhost".to_string();
        }

        if self.address.contains(':') && !self.address.starts_with('[') {
            format!("[{}]", self.address)
        } else {
            self.address.clone()
        }
    }

    fn same_process_binding(&self, other: &Self) -> bool {
        self.address == other.address
            && self.port == other.port
            && self.pid == other.pid
            && self.process == other.process
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    theme: ThemeConfig,
    #[serde(default)]
    display: DisplayConfig,
    #[serde(default)]
    ports: PortsConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeConfig {
    #[serde(default = "default_preset")]
    preset: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            preset: default_preset(),
        }
    }
}

fn default_preset() -> String {
    "dark".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisplayConfig {
    #[serde(default = "default_refresh")]
    refresh_interval: u64,
    #[serde(default = "default_true")]
    show_pid: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            refresh_interval: default_refresh(),
            show_pid: true,
        }
    }
}

fn default_refresh() -> u64 {
    2
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct PortsConfig {
    #[serde(default)]
    exclude: Vec<u16>,
    #[serde(default)]
    include: Vec<[u16; 2]>,
    #[serde(default)]
    include_wildcard: bool,
}

fn config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(path) = std::env::var_os("LOCALHOSTEL_CONFIG") {
        paths.push(PathBuf::from(path));
    }
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join(APP_NAME).join("config.toml"));
    }
    if let Some(home_dir) = dirs::home_dir() {
        paths.push(home_dir.join(".localhostel.toml"));
    }

    paths
}

fn load_config() -> Config {
    load_config_from_paths(config_paths())
}

fn load_config_from_paths<I, P>(paths: I) -> Config
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    for path in paths {
        let path = path.as_ref();
        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => return config,
                Err(err) => eprintln!("Ignoring invalid config {}: {err}", path.display()),
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => eprintln!("Unable to read config {}: {err}", path.display()),
        }
    }

    Config::default()
}

fn scan_ports(config: &Config) -> Vec<PortEntry> {
    let mut entries: HashMap<(String, u16), PortEntry> = HashMap::new();

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("lsof")
            .args(["-iTCP", "-sTCP:LISTEN", "-nP"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                if let Some(entry) = parse_lsof_line(line)
                    && should_include_entry(&entry, config)
                {
                    entries
                        .entry((entry.address.clone(), entry.port))
                        .or_insert(entry);
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("ss").args(["-tlnp"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                if let Some(entry) = parse_ss_line(line)
                    && should_include_entry(&entry, config)
                {
                    entries
                        .entry((entry.address.clone(), entry.port))
                        .or_insert(entry);
                }
            }
        }
    }

    let mut result: Vec<_> = entries.into_values().collect();
    result.sort_by(|left, right| {
        left.port
            .cmp(&right.port)
            .then_with(|| left.address.cmp(&right.address))
    });
    result
}

fn should_include_entry(entry: &PortEntry, config: &Config) -> bool {
    if config.ports.exclude.contains(&entry.port) {
        return false;
    }

    if !config.ports.include.is_empty()
        && !config
            .ports
            .include
            .iter()
            .any(|[start, end]| start <= end && entry.port >= *start && entry.port <= *end)
    {
        return false;
    }

    entry.is_loopback() || (config.ports.include_wildcard && entry.is_wildcard())
}

#[cfg(any(target_os = "macos", test))]
fn parse_lsof_line(line: &str) -> Option<PortEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        return None;
    }

    let process = parts[0].to_string();
    let pid = parts[1].parse::<u32>().ok().filter(|pid| *pid > 0)?;
    let (address, port) = parse_endpoint(parts[8])?;

    Some(PortEntry {
        address,
        port,
        pid: Some(pid),
        process,
    })
}

#[cfg(any(target_os = "linux", test))]
fn parse_ss_line(line: &str) -> Option<PortEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let (address, port) = parse_endpoint(parts[3])?;
    let (process, pid) = parts
        .get(5)
        .map(|users| parse_ss_users(users))
        .unwrap_or_else(|| ("unknown".to_string(), None));

    Some(PortEntry {
        address,
        port,
        pid,
        process,
    })
}

#[cfg(any(target_os = "linux", test))]
fn parse_ss_users(users: &str) -> (String, Option<u32>) {
    let process = users
        .split('"')
        .nth(1)
        .filter(|name| !name.is_empty())
        .unwrap_or("unknown")
        .to_string();

    let pid = users.find("pid=").and_then(|start| {
        let digits: String = users[start + 4..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();

        digits.parse::<u32>().ok().filter(|pid| *pid > 0)
    });

    (process, pid)
}

fn parse_endpoint(endpoint: &str) -> Option<(String, u16)> {
    let endpoint = endpoint.trim();
    let colon_pos = endpoint.rfind(':')?;
    let (host, port) = endpoint.split_at(colon_pos);
    let port = port.trim_start_matches(':').parse().ok()?;
    let host = normalize_host(host);

    if host.is_empty() {
        return None;
    }

    Some((host, port))
}

fn normalize_host(host: &str) -> String {
    host.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

fn is_loopback_host(host: &str) -> bool {
    let host = normalize_host(host);
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    match host.parse::<IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        Err(_) => false,
    }
}

fn is_wildcard_host(host: &str) -> bool {
    matches!(normalize_host(host).as_str(), "*" | "0.0.0.0" | "::")
}

fn open_in_browser(url: &str) -> bool {
    let Some(mut command) = browser_command() else {
        return false;
    };

    command.arg(url).spawn().map(|_| true).unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn browser_command() -> Option<Command> {
    Some(Command::new("open"))
}

#[cfg(target_os = "linux")]
fn browser_command() -> Option<Command> {
    Some(Command::new("xdg-open"))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn browser_command() -> Option<Command> {
    None
}

fn kill_process(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }

    #[cfg(unix)]
    {
        Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn copy_to_clipboard(text: &str) -> bool {
    let Some(mut command) = clipboard_command() else {
        return false;
    };

    copy_with_command(&mut command, text)
}

#[cfg(target_os = "macos")]
fn clipboard_command() -> Option<Command> {
    Some(Command::new("pbcopy"))
}

#[cfg(target_os = "linux")]
fn clipboard_command() -> Option<Command> {
    let mut command = Command::new("xclip");
    command.args(["-selection", "clipboard"]);
    Some(command)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn clipboard_command() -> Option<Command> {
    None
}

fn copy_with_command(command: &mut Command, text: &str) -> bool {
    let Ok(mut child) = command.stdin(Stdio::piped()).spawn() else {
        return false;
    };

    if let Some(stdin) = child.stdin.as_mut()
        && stdin.write_all(text.as_bytes()).is_err()
    {
        return false;
    }

    child.wait().map(|status| status.success()).unwrap_or(false)
}

#[derive(Debug, PartialEq, Eq)]
enum KillError {
    MissingPid,
    StaleEntry,
    SignalFailed,
}

fn kill_entry(entry: &PortEntry, config: &Config) -> Result<(), KillError> {
    let pid = entry.pid.ok_or(KillError::MissingPid)?;
    let still_matches = scan_ports(config)
        .iter()
        .any(|current| current.same_process_binding(entry));

    if !still_matches {
        return Err(KillError::StaleEntry);
    }

    if kill_process(pid) {
        Ok(())
    } else {
        Err(KillError::SignalFailed)
    }
}

fn pid_label(pid: Option<u32>) -> String {
    pid.map(|pid| pid.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn app_title() -> String {
    format!("{APP_NAME} {PRODUCT_VERSION}")
}

fn version_text() -> String {
    format!("{APP_NAME} {PRODUCT_VERSION}")
}

fn should_print_version() -> bool {
    std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
}

#[Window(events = CommandBarEvents + TimerEvents + ListBoxEvents, commands: [Open, Kill, Copy, Refresh, Quit])]
struct Localhostel {
    list: Handle<ListBox>,
    status: Handle<Label>,
    entries: Vec<PortEntry>,
    config: Config,
    selected_idx: usize,
}

impl Localhostel {
    fn new(config: Config) -> Self {
        let refresh_interval = config.display.refresh_interval.max(1);

        let mut w = Self {
            base: window!("'localhostel',a:c,w:78,h:22,flags:Sizeable"),
            list: Handle::None,
            status: Handle::None,
            entries: Vec::new(),
            config,
            selected_idx: 0,
        };

        w.set_title(&app_title());
        w.add(label!(
            "' ADDRESS           PORT      PID  PROCESS',l:1,t:0,w:76"
        ));

        let mut panel = panel!("l:0,t:1,r:0,b:2,type:TopBar");
        w.list = panel.add(listbox!(
            "d:f,flags: ScrollBars+HighlightSelectedItemWhenInactive"
        ));
        w.add(panel);

        w.status = w.add(label!("'  Ready',l:0,b:0,r:0"));
        w.show_status(&format!("  Ready - {}", app_title()));

        w.refresh_list();

        if let Some(timer) = w.timer() {
            timer.start(Duration::from_secs(refresh_interval));
        }

        w
    }

    fn refresh_list(&mut self) {
        self.entries = scan_ports(&self.config);
        if self.entries.is_empty() {
            self.selected_idx = 0;
        } else if self.selected_idx >= self.entries.len() {
            self.selected_idx = self.entries.len() - 1;
        }

        let show_pid = self.config.display.show_pid;
        let list_handle = self.list;

        let lines: Vec<String> = if self.entries.is_empty() {
            vec!["  No loopback sessions found".to_string()]
        } else {
            self.entries
                .iter()
                .map(|entry| {
                    if show_pid {
                        format!(
                            " {:<16} :{:<5} {:>6}  {}",
                            entry.address,
                            entry.port,
                            pid_label(entry.pid),
                            entry.process
                        )
                    } else {
                        format!(
                            " {:<16} :{:<5} {}",
                            entry.address, entry.port, entry.process
                        )
                    }
                })
                .collect()
        };

        if let Some(lb) = self.control_mut(list_handle) {
            lb.clear();
            for line in lines {
                lb.add(&line);
            }
        }
    }

    fn get_selected_entry(&self) -> Option<PortEntry> {
        self.entries.get(self.selected_idx).cloned()
    }

    fn show_status(&mut self, msg: &str) {
        let handle = self.status;
        if let Some(lbl) = self.control_mut(handle) {
            lbl.set_caption(msg);
        }
    }
}

impl CommandBarEvents for Localhostel {
    fn on_update_commandbar(&self, commandbar: &mut CommandBar) {
        commandbar.set(key!("Enter"), "Open", localhostel::Commands::Open);
        commandbar.set(key!("k"), "Kill", localhostel::Commands::Kill);
        commandbar.set(key!("c"), "Copy", localhostel::Commands::Copy);
        commandbar.set(key!("r"), "Refresh", localhostel::Commands::Refresh);
        commandbar.set(key!("q"), "Quit", localhostel::Commands::Quit);
    }

    fn on_event(&mut self, command_id: localhostel::Commands) {
        match command_id {
            localhostel::Commands::Open => {
                if let Some(entry) = self.get_selected_entry() {
                    let url = entry.url();
                    if open_in_browser(&url) {
                        self.show_status(&format!("  Opened {url}"));
                    } else {
                        self.show_status(&format!("  Failed to open {url}"));
                    }
                }
            }
            localhostel::Commands::Kill => {
                if let Some(entry) = self.get_selected_entry() {
                    match kill_entry(&entry, &self.config) {
                        Ok(()) => {
                            self.show_status(&format!(
                                "  Sent SIGTERM to {} ({}:{})",
                                pid_label(entry.pid),
                                entry.address,
                                entry.port
                            ));
                            self.refresh_list();
                        }
                        Err(KillError::MissingPid) => {
                            self.show_status("  Cannot kill: process ID is unknown");
                        }
                        Err(KillError::StaleEntry) => {
                            self.show_status("  Refreshed: selected process changed");
                            self.refresh_list();
                        }
                        Err(KillError::SignalFailed) => {
                            self.show_status(&format!(
                                "  Failed to signal PID {}",
                                pid_label(entry.pid)
                            ));
                        }
                    }
                }
            }
            localhostel::Commands::Copy => {
                if let Some(entry) = self.get_selected_entry() {
                    let url = entry.url();
                    if copy_to_clipboard(&url) {
                        self.show_status(&format!("  Copied {url}"));
                    } else {
                        self.show_status("  Failed to copy URL");
                    }
                }
            }
            localhostel::Commands::Refresh => {
                self.refresh_list();
                self.show_status(&format!("  Refreshed - {} sessions", self.entries.len()));
            }
            localhostel::Commands::Quit => {
                self.close();
            }
        }
    }
}

impl TimerEvents for Localhostel {
    fn on_update(&mut self, _ticks: u64) -> EventProcessStatus {
        self.refresh_list();
        EventProcessStatus::Processed
    }
}

impl ListBoxEvents for Localhostel {
    fn on_current_item_changed(
        &mut self,
        _handle: Handle<ListBox>,
        index: usize,
    ) -> EventProcessStatus {
        self.selected_idx = index;
        EventProcessStatus::Processed
    }
}

fn main() -> Result<(), appcui::system::Error> {
    if should_print_version() {
        println!("{}", version_text());
        return Ok(());
    }

    let _guard = match acquire_lock() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("Could not start {APP_NAME}: {err}");
            return Ok(());
        }
    };

    let config = load_config();

    let theme = match config.theme.preset.as_str() {
        "light" => Themes::Light,
        "midnight" | "dark" => Themes::DarkGray,
        _ => Themes::Default,
    };

    let mut app = App::new().theme(Theme::new(theme)).build()?;

    app.add_window(Localhostel::new(config));
    app.run();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(address: &str, port: u16, pid: Option<u32>) -> PortEntry {
        PortEntry {
            address: address.to_string(),
            port,
            pid,
            process: "node".to_string(),
        }
    }

    #[test]
    fn includes_loopback_ports_by_default() {
        let config = Config::default();
        assert!(should_include_entry(
            &entry("127.0.0.1", 3000, Some(123)),
            &config
        ));
        assert!(should_include_entry(
            &entry("::1", 5174, Some(123)),
            &config
        ));
    }

    #[test]
    fn excludes_wildcard_ports_by_default() {
        let config = Config::default();
        assert!(!should_include_entry(&entry("*", 3000, Some(123)), &config));
        assert!(!should_include_entry(
            &entry("0.0.0.0", 3000, Some(123)),
            &config
        ));
    }

    #[test]
    fn can_include_wildcard_ports_explicitly() {
        let config = Config {
            ports: PortsConfig {
                include_wildcard: true,
                ..PortsConfig::default()
            },
            ..Config::default()
        };

        assert!(should_include_entry(&entry("*", 3000, Some(123)), &config));
    }

    #[test]
    fn excludes_configured_ports_before_address_filtering() {
        let config = Config {
            ports: PortsConfig {
                exclude: vec![3000],
                include_wildcard: true,
                ..PortsConfig::default()
            },
            ..Config::default()
        };

        assert!(!should_include_entry(
            &entry("127.0.0.1", 3000, Some(123)),
            &config
        ));
        assert!(!should_include_entry(&entry("*", 3000, Some(123)), &config));
    }

    #[test]
    fn include_ranges_must_match_port() {
        let config = Config {
            ports: PortsConfig {
                include: vec![[8000, 8999]],
                ..PortsConfig::default()
            },
            ..Config::default()
        };

        assert!(should_include_entry(
            &entry("127.0.0.1", 8080, Some(123)),
            &config
        ));
        assert!(!should_include_entry(
            &entry("127.0.0.1", 3000, Some(123)),
            &config
        ));
    }

    #[test]
    fn parses_lsof_loopback_line() {
        let line = "node 78209 ama 13u IPv6 0x8941e63ca2d2745a 0t0 TCP [::1]:5174 (LISTEN)";
        let parsed = parse_lsof_line(line).expect("lsof line should parse");

        assert_eq!(parsed.address, "::1");
        assert_eq!(parsed.port, 5174);
        assert_eq!(parsed.pid, Some(78209));
        assert_eq!(parsed.process, "node");
    }

    #[test]
    fn parses_lsof_wildcard_line() {
        let line = "ControlCe 1039 ama 8u IPv4 0x6f9ee1c6cb79fde5 0t0 TCP *:7000 (LISTEN)";
        let parsed = parse_lsof_line(line).expect("lsof wildcard line should parse");

        assert_eq!(parsed.address, "*");
        assert_eq!(parsed.port, 7000);
        assert_eq!(parsed.pid, Some(1039));
    }

    #[test]
    fn parses_ss_line_without_pid_as_unknown_pid() {
        let line = "LISTEN 0 4096 127.0.0.1:3000 0.0.0.0:*";
        let parsed = parse_ss_line(line).expect("ss line should parse");

        assert_eq!(parsed.address, "127.0.0.1");
        assert_eq!(parsed.port, 3000);
        assert_eq!(parsed.pid, None);
        assert_eq!(parsed.process, "unknown");
    }

    #[test]
    fn parses_ss_users_process_and_pid() {
        let (process, pid) = parse_ss_users("users:((\"node\",pid=78209,fd=13))");

        assert_eq!(process, "node");
        assert_eq!(pid, Some(78209));
    }

    #[test]
    fn parses_ipv6_endpoints() {
        assert_eq!(
            parse_endpoint("[::1]:5174"),
            Some(("::1".to_string(), 5174))
        );
    }

    #[test]
    fn unknown_config_fields_are_rejected() {
        let err = toml::from_str::<Config>("[display]\nshow_command = true\n")
            .expect_err("show_command should not be accepted");

        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn version_uses_product_display_scheme() {
        assert_eq!(version_text(), "localhostel 0.1000");
        assert_eq!(app_title(), "localhostel 0.1000");
    }
}
