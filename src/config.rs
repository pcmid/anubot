use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bot_token: String,
    pub database_url: String,
    pub public_url: String,
    pub listen_addr: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("environment variable {0} is required")]
    Missing(&'static str),
    #[error("environment variable {name} is invalid: {reason}")]
    Invalid {
        name: &'static str,
        reason: &'static str,
    },
}

pub const DEFAULT_DATABASE_URL: &str = "sqlite:anubot.sqlite";
pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8080";

impl Config {
    pub fn from_getter<F>(get: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let bot_token = get("BOT_TOKEN").ok_or(ConfigError::Missing("BOT_TOKEN"))?;
        if bot_token.is_empty() {
            return Err(ConfigError::Invalid {
                name: "BOT_TOKEN",
                reason: "must not be empty",
            });
        }
        let public_url = get("PUBLIC_URL").ok_or(ConfigError::Missing("PUBLIC_URL"))?;
        if public_url.is_empty() {
            return Err(ConfigError::Invalid {
                name: "PUBLIC_URL",
                reason: "must not be empty",
            });
        }
        if Url::parse(&public_url).is_err() {
            return Err(ConfigError::Invalid {
                name: "PUBLIC_URL",
                reason: "must be an absolute URL with scheme (e.g. https://bot.example.com)",
            });
        }
        let database_url = get("DATABASE_URL").unwrap_or_else(|| DEFAULT_DATABASE_URL.to_string());
        let listen_addr = get("LISTEN_ADDR").unwrap_or_else(|| DEFAULT_LISTEN_ADDR.to_string());
        Ok(Self {
            bot_token,
            database_url,
            public_url,
            listen_addr,
        })
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_getter(|k| std::env::var(k).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn getter(map: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = map
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn accepts_valid_public_url() {
        let cfg = Config::from_getter(getter(&[
            ("BOT_TOKEN", "token"),
            ("PUBLIC_URL", "https://bot.example.com"),
        ]))
        .unwrap();
        assert_eq!(cfg.public_url, "https://bot.example.com");
    }

    #[test]
    fn rejects_public_url_without_scheme() {
        let err = Config::from_getter(getter(&[
            ("BOT_TOKEN", "token"),
            ("PUBLIC_URL", "bot.example.com"),
        ]))
        .unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                name: "PUBLIC_URL",
                ..
            }
        ));
    }
}
