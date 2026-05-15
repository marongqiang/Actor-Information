use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct CommandError {
    pub code: u16,
    pub message: String,
}

impl CommandError {
    pub fn db(msg: &str) -> Self { Self { code: 1000, message: msg.to_string() } }
    pub fn network(msg: &str) -> Self { Self { code: 2000, message: msg.to_string() } }
    pub fn unauthorized(msg: &str) -> Self { Self { code: 2100, message: msg.to_string() } }
    pub fn rate_limited(msg: &str) -> Self { Self { code: 2200, message: msg.to_string() } }
    pub fn invalid_input(msg: &str) -> Self { Self { code: 3000, message: msg.to_string() } }
    pub fn not_found(msg: &str) -> Self { Self { code: 3100, message: msg.to_string() } }
    pub fn internal(msg: &str) -> Self { Self { code: 4000, message: msg.to_string() } }
    pub fn scrape_failed(msg: &str) -> Self { Self { code: 4100, message: msg.to_string() } }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for CommandError {}

impl From<rusqlite::Error> for CommandError {
    fn from(e: rusqlite::Error) -> Self {
        Self::db(&format!("数据库错误: {}", e))
    }
}

impl From<reqwest::Error> for CommandError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::network("请求超时")
        } else {
            Self::network(&format!("网络错误: {}", e))
        }
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(e: serde_json::Error) -> Self {
        Self::internal(&format!("序列化错误: {}", e))
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
