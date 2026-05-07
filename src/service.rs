pub const MIN_SERVICE_PORT: u16 = 1024;
pub const MAX_SERVICE_PORT: u16 = 9999;
pub const MEMO_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalService {
    pub pid: u32,
    pub port: u16,
    pub address: String,
    pub process_name: String,
    pub command: String,
    pub memo: Option<String>,
}

impl LocalService {
    pub fn memo_key(&self) -> String {
        memo_key(self.pid, self.port, &self.process_name)
    }

    pub fn display_name(&self) -> &str {
        if self.process_name.trim().is_empty() {
            "unknown"
        } else {
            self.process_name.trim()
        }
    }
}

pub fn memo_key(pid: u32, port: u16, process_name: &str) -> String {
    format!("{pid}:{port}:{}", process_name.trim())
}

pub fn normalize_memo(input: &str) -> Option<String> {
    let memo = input
        .chars()
        .filter(|ch| *ch != '\n' && *ch != '\r')
        .take(MEMO_LIMIT)
        .collect::<String>()
        .trim()
        .to_string();

    if memo.is_empty() {
        None
    } else {
        Some(memo)
    }
}

pub fn is_service_port(port: u16) -> bool {
    (MIN_SERVICE_PORT..=MAX_SERVICE_PORT).contains(&port)
}

pub fn is_safe_pid(pid: u32) -> bool {
    pid > 1
}

#[cfg(test)]
mod tests {
    use super::{is_safe_pid, memo_key, normalize_memo, MEMO_LIMIT};

    #[test]
    fn memo_key_is_stable_for_live_service_identity() {
        assert_eq!(memo_key(42, 5173, " node "), "42:5173:node");
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
}
