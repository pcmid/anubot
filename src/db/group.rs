pub(crate) use crate::db::repo::group::*;

use serde::{Deserialize, Serialize};

pub const DEFAULT_TIMEOUT_SECONDS: i64 = 600;
pub const DEFAULT_SPAM_CHECK_MESSAGE_LIMIT: i64 = 3;
pub const DEFAULT_SPAM_CHECK_WINDOW_HOURS: i64 = 24;
pub const DEFAULT_SPAM_KICK_THRESHOLD: i64 = 2;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: Option<String>,
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub spam_check_message_limit: Option<i64>,
    pub spam_check_window_hours: Option<i64>,
    pub spam_kick_threshold: Option<i64>,
}

impl AiConfig {
    pub fn ready(&self) -> Option<(&str, &str, &str, &str)> {
        Some((
            self.provider.as_deref()?,
            self.api_base.as_deref()?,
            self.api_key.as_deref()?,
            self.model.as_deref()?,
        ))
    }

    pub fn spam_check_message_limit(&self) -> i64 {
        self.spam_check_message_limit
            .unwrap_or(DEFAULT_SPAM_CHECK_MESSAGE_LIMIT)
    }

    pub fn spam_check_window_hours(&self) -> i64 {
        self.spam_check_window_hours
            .unwrap_or(DEFAULT_SPAM_CHECK_WINDOW_HOURS)
    }

    pub fn spam_kick_threshold(&self) -> i64 {
        self.spam_kick_threshold
            .unwrap_or(DEFAULT_SPAM_KICK_THRESHOLD)
    }
}
