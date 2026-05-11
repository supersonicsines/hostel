use serde::{Deserialize, Serialize};

pub const MIN_SERVICE_PORT: u16 = 1024;
pub const MAX_SERVICE_PORT: u16 = 9999;
pub const MEMO_LIMIT: usize = 100;
pub const TITLE_LIMIT: usize = 80;
pub const URL_PATH_LIMIT: usize = 160;
pub const SOURCE_LIMIT: usize = 60;
pub const TAG_LIMIT: usize = 32;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_unix: Option<u64>,
}

impl ServiceMetadata {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.memo.is_none()
            && self.tags.is_empty()
            && self.url_path.is_none()
            && self.scheme.is_none()
            && self.source.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalService {
    pub pid: u32,
    pub port: u16,
    pub address: String,
    pub process_name: String,
    pub command: String,
    pub kind: ServiceKind,
    pub metadata: ServiceMetadata,
}

impl LocalService {
    pub fn memo_key(&self) -> String {
        memo_key(self.pid, self.port, &self.process_name)
    }

    pub fn metadata_key(&self) -> String {
        metadata_key(self.port, &self.process_name, &self.command)
    }

    pub fn display_name(&self) -> &str {
        if self.process_name.trim().is_empty() {
            "unknown"
        } else {
            self.process_name.trim()
        }
    }

    pub fn display_title(&self) -> &str {
        self.metadata
            .title
            .as_deref()
            .unwrap_or_else(|| self.display_name())
    }

    pub fn matches_keyword(&self, keyword: &str) -> bool {
        let keyword = keyword.trim().to_lowercase();
        if keyword.is_empty() {
            return false;
        }

        self.port.to_string().contains(&keyword)
            || self.address.to_lowercase().contains(&keyword)
            || self.process_name.to_lowercase().contains(&keyword)
            || self.command.to_lowercase().contains(&keyword)
            || self
                .kind
                .label()
                .is_some_and(|label| label.to_lowercase().contains(&keyword))
            || self
                .metadata
                .title
                .as_ref()
                .is_some_and(|title| title.to_lowercase().contains(&keyword))
            || self
                .metadata
                .memo
                .as_ref()
                .is_some_and(|memo| memo.to_lowercase().contains(&keyword))
            || self
                .metadata
                .url_path
                .as_ref()
                .is_some_and(|path| path.to_lowercase().contains(&keyword))
            || self
                .metadata
                .source
                .as_ref()
                .is_some_and(|source| source.to_lowercase().contains(&keyword))
            || self
                .metadata
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&keyword))
    }

    pub fn is_hidden_by(&self, keywords: &[String]) -> bool {
        keywords.iter().any(|keyword| self.matches_keyword(keyword))
    }

    pub fn open_url(&self) -> String {
        let scheme = self.metadata.scheme.as_deref().unwrap_or("http");
        format!(
            "{scheme}://localhost:{}{}",
            self.port,
            self.metadata.url_path.as_deref().unwrap_or("/")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    Astro,
    Vite,
    Next,
    Nuxt,
    Storybook,
    Api,
    Rust,
    Python,
    Node,
    Unknown,
}

impl ServiceKind {
    pub fn label(self) -> Option<&'static str> {
        match self {
            ServiceKind::Astro => Some("Astro"),
            ServiceKind::Vite => Some("Vite"),
            ServiceKind::Next => Some("Next"),
            ServiceKind::Nuxt => Some("Nuxt"),
            ServiceKind::Storybook => Some("Storybook"),
            ServiceKind::Api => Some("API"),
            ServiceKind::Rust => Some("Rust"),
            ServiceKind::Python => Some("Python"),
            ServiceKind::Node => Some("Node"),
            ServiceKind::Unknown => None,
        }
    }
}

pub fn detect_service_kind(process_name: &str, command: &str) -> ServiceKind {
    let haystack = format!("{process_name} {command}").to_lowercase();

    if haystack.contains("storybook") {
        ServiceKind::Storybook
    } else if haystack.contains("astro") {
        ServiceKind::Astro
    } else if haystack.contains("vite") {
        ServiceKind::Vite
    } else if haystack.contains("next") {
        ServiceKind::Next
    } else if haystack.contains("nuxt") {
        ServiceKind::Nuxt
    } else if haystack.contains("uvicorn")
        || haystack.contains("fastapi")
        || haystack.contains("flask")
        || haystack.contains("django")
        || haystack.contains("rails")
        || haystack.contains("puma")
    {
        ServiceKind::Api
    } else if haystack.contains("cargo") || haystack.contains("rust") {
        ServiceKind::Rust
    } else if haystack.contains("python") || haystack.contains("python3") {
        ServiceKind::Python
    } else if haystack.contains("node")
        || haystack.contains("npm")
        || haystack.contains("pnpm")
        || haystack.contains("yarn")
        || haystack.contains("bun")
        || haystack.contains("deno")
    {
        ServiceKind::Node
    } else {
        ServiceKind::Unknown
    }
}

pub fn memo_key(pid: u32, port: u16, process_name: &str) -> String {
    format!("{pid}:{port}:{}", process_name.trim())
}

pub fn metadata_key(port: u16, process_name: &str, command: &str) -> String {
    let process_name = normalize_identity_part(process_name);
    let command = normalize_identity_part(command);
    format!(
        "v1:{port}:{process_name}:{:016x}",
        stable_hash(command.as_bytes())
    )
}

fn normalize_identity_part(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn normalize_title(input: &str) -> Option<String> {
    normalize_limited_line(input, TITLE_LIMIT)
}

pub fn normalize_memo(input: &str) -> Option<String> {
    normalize_limited_line(input, MEMO_LIMIT)
}

pub fn normalize_source(input: &str) -> Option<String> {
    normalize_limited_line(input, SOURCE_LIMIT)
}

pub fn normalize_scheme(input: &str) -> Option<String> {
    match input.trim().to_lowercase().as_str() {
        "" => None,
        "http" => Some("http".to_string()),
        "https" => Some("https".to_string()),
        _ => None,
    }
}

fn normalize_limited_line(input: &str, limit: usize) -> Option<String> {
    let value = input
        .chars()
        .filter(|ch| *ch != '\n' && *ch != '\r')
        .take(limit)
        .collect::<String>()
        .trim()
        .to_string();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn normalize_url_path(input: &str) -> Option<String> {
    let path = input
        .chars()
        .filter(|ch| *ch != '\n' && *ch != '\r')
        .take(URL_PATH_LIMIT)
        .collect::<String>()
        .trim()
        .to_string();

    if path.is_empty() {
        None
    } else if path.starts_with('/') || path.starts_with('?') || path.starts_with('#') {
        Some(path)
    } else {
        Some(format!("/{path}"))
    }
}

pub fn is_service_port(port: u16) -> bool {
    (MIN_SERVICE_PORT..=MAX_SERVICE_PORT).contains(&port)
}

pub fn is_safe_pid(pid: u32) -> bool {
    pid > 1
}

pub fn normalize_filter_keywords(input: &str) -> Vec<String> {
    let mut keywords = input
        .split(',')
        .map(|keyword| keyword.trim().to_lowercase())
        .filter(|keyword| !keyword.is_empty())
        .collect::<Vec<_>>();

    keywords.sort();
    keywords.dedup();
    keywords
}

pub fn normalize_tags(values: &[String]) -> Vec<String> {
    let mut tags = values
        .iter()
        .flat_map(|value| value.split(','))
        .map(|tag| {
            tag.chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || matches!(*ch, '-' | '_' | '.'))
                .take(TAG_LIMIT)
                .collect::<String>()
                .trim_matches(['-', '_', '.'])
                .to_lowercase()
        })
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();

    tags.sort();
    tags.dedup();
    tags
}

#[cfg(test)]
mod tests {
    use super::{
        detect_service_kind, is_safe_pid, memo_key, metadata_key, normalize_filter_keywords,
        normalize_memo, normalize_scheme, normalize_tags, normalize_title, normalize_url_path,
        LocalService, ServiceKind, ServiceMetadata, MEMO_LIMIT, TITLE_LIMIT,
    };

    #[test]
    fn memo_key_is_stable_for_live_service_identity() {
        assert_eq!(memo_key(42, 5173, " node "), "42:5173:node");
    }

    #[test]
    fn metadata_key_uses_stable_service_fingerprint() {
        assert_eq!(
            metadata_key(5173, " node ", " npm   run dev "),
            metadata_key(5173, "node", "npm run dev")
        );
        assert_ne!(
            metadata_key(5173, "node", "npm run dev"),
            metadata_key(5174, "node", "npm run dev")
        );
    }

    #[test]
    fn title_limit_is_enforced() {
        let input = "x".repeat(TITLE_LIMIT + 20);
        assert_eq!(normalize_title(&input).expect("title").len(), TITLE_LIMIT);
    }

    #[test]
    fn memo_limit_is_enforced() {
        let input = "x".repeat(MEMO_LIMIT + 20);
        assert_eq!(normalize_memo(&input).expect("memo").len(), MEMO_LIMIT);
    }

    #[test]
    fn empty_memo_clears() {
        assert_eq!(normalize_memo("   \n"), None);
    }

    #[test]
    fn pid_zero_and_one_are_not_safe() {
        assert!(!is_safe_pid(0));
        assert!(!is_safe_pid(1));
        assert!(is_safe_pid(2));
    }

    #[test]
    fn filter_keywords_are_normalized() {
        assert_eq!(
            normalize_filter_keywords(" node, , VITE, node "),
            vec!["node".to_string(), "vite".to_string()]
        );
    }

    #[test]
    fn service_hides_when_keyword_matches_process_command_port_or_memo() {
        let service = LocalService {
            pid: 12,
            port: 5173,
            address: "127.0.0.1".to_string(),
            process_name: "node".to_string(),
            command: "npm run dev".to_string(),
            kind: ServiceKind::Vite,
            metadata: ServiceMetadata {
                title: Some("Checkout".to_string()),
                memo: Some("frontend".to_string()),
                tags: vec!["agent".to_string()],
                ..ServiceMetadata::default()
            },
        };

        assert!(service.is_hidden_by(&["node".to_string()]));
        assert!(service.is_hidden_by(&["5173".to_string()]));
        assert!(service.is_hidden_by(&["run dev".to_string()]));
        assert!(service.is_hidden_by(&["front".to_string()]));
        assert!(service.is_hidden_by(&["checkout".to_string()]));
        assert!(service.is_hidden_by(&["agent".to_string()]));
        assert!(!service.is_hidden_by(&["python".to_string()]));
    }

    #[test]
    fn detects_common_service_badges() {
        assert_eq!(detect_service_kind("node", "astro dev"), ServiceKind::Astro);
        assert_eq!(
            detect_service_kind("node", "vite --host 127.0.0.1"),
            ServiceKind::Vite
        );
        assert_eq!(detect_service_kind("node", "next dev"), ServiceKind::Next);
        assert_eq!(
            detect_service_kind("python3", "uvicorn app:app"),
            ServiceKind::Api
        );
        assert_eq!(detect_service_kind("cargo", "cargo run"), ServiceKind::Rust);
    }

    #[test]
    fn url_paths_are_normalized() {
        assert_eq!(normalize_url_path("docs"), Some("/docs".to_string()));
        assert_eq!(normalize_url_path("/admin"), Some("/admin".to_string()));
        assert_eq!(
            normalize_url_path("?debug=true"),
            Some("?debug=true".to_string())
        );
        assert_eq!(normalize_url_path("  "), None);
    }

    #[test]
    fn schemes_are_limited_to_http_variants() {
        assert_eq!(normalize_scheme("HTTPS"), Some("https".to_string()));
        assert_eq!(normalize_scheme("ftp"), None);
        assert_eq!(normalize_scheme(" "), None);
    }

    #[test]
    fn tags_are_sanitized_sorted_and_deduped() {
        assert_eq!(
            normalize_tags(&[" Vite,Front End ".to_string(), "vite".to_string()]),
            vec!["frontend".to_string(), "vite".to_string()]
        );
    }

    #[test]
    fn open_url_uses_override_path() {
        let mut service = LocalService {
            pid: 12,
            port: 8000,
            address: "127.0.0.1".to_string(),
            process_name: "python3".to_string(),
            command: "uvicorn app:app".to_string(),
            kind: ServiceKind::Api,
            metadata: ServiceMetadata::default(),
        };
        assert_eq!(service.open_url(), "http://localhost:8000/");

        service.metadata.url_path = Some("/docs".to_string());
        assert_eq!(service.open_url(), "http://localhost:8000/docs");
        service.metadata.scheme = Some("https".to_string());
        assert_eq!(service.open_url(), "https://localhost:8000/docs");
    }
}
