use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Directory (relative to the platform base) that stores the config file.
const APP_DIR: &str = "telegram-pc-bot";
/// File name of the persisted configuration.
const CONFIG_FILE: &str = "config.json";

/// Persisted runtime configuration.
///
/// The same structure is written by the interactive setup wizard and read on
/// every subsequent (including auto-started) launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Telegram bot token issued by @BotFather.
    pub bot_token: String,
    /// Numeric Telegram user ids allowed to control the host.
    pub allowed_user_ids: Vec<u64>,
}

impl Config {
    /// Absolute path of the persisted configuration file.
    ///
    /// Uses `%ProgramData%` on Windows so an auto-started SYSTEM task and the
    /// interactive user read the same file. Falls back to the user config dir
    /// on other platforms (useful for development).
    #[must_use]
    pub fn dir() -> PathBuf {
        let base = if cfg!(windows) {
            env::var_os("PROGRAMDATA").map_or_else(env::temp_dir, PathBuf::from)
        } else {
            env::var_os("XDG_CONFIG_HOME").map_or_else(
                || {
                    env::var_os("HOME")
                        .map_or_else(env::temp_dir, |home| PathBuf::from(home).join(".config"))
                },
                PathBuf::from,
            )
        };
        base.join(APP_DIR)
    }

    /// Absolute path of the persisted configuration file.
    #[must_use]
    pub fn path() -> PathBuf {
        Self::dir().join(CONFIG_FILE)
    }

    /// Whether a configuration file already exists on disk.
    #[must_use]
    pub fn exists() -> bool {
        Self::path().is_file()
    }

    /// Load and validate the configuration from disk.
    ///
    /// # Errors
    /// Returns [`AppError::Config`] when the file is missing, unreadable, or
    /// does not contain a usable token and at least one user id.
    pub fn load() -> AppResult<Self> {
        let path = Self::path();
        let raw = fs::read_to_string(&path).map_err(|error| {
            AppError::Config(format!("cannot read {}: {error}", path.display()))
        })?;
        let config: Self = serde_json::from_str(&raw).map_err(|error| {
            AppError::Config(format!("invalid config at {}: {error}", path.display()))
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Persist the configuration, creating the parent directory if needed.
    ///
    /// # Errors
    /// Returns [`AppError::Config`] when validation fails or the file cannot
    /// be written.
    pub fn save(&self) -> AppResult<()> {
        self.validate()?;
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AppError::Config(format!("cannot create {}: {error}", parent.display()))
            })?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| AppError::Config(format!("cannot serialize config: {error}")))?;
        fs::write(&path, json)
            .map_err(|error| AppError::Config(format!("cannot write {}: {error}", path.display())))
    }

    /// Remove the persisted configuration file if it exists.
    ///
    /// # Errors
    /// Returns [`AppError::Io`] when the file exists but cannot be deleted.
    pub fn remove() -> AppResult<()> {
        let path = Self::path();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// The Telegram bot token.
    #[must_use]
    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }

    /// Whether `user_id` is permitted to control the host.
    #[must_use]
    pub fn is_allowed(&self, user_id: u64) -> bool {
        self.allowed_user_ids.contains(&user_id)
    }

    /// Number of whitelisted Telegram user ids.
    #[must_use]
    pub fn allowed_user_count(&self) -> usize {
        self.allowed_user_ids.len()
    }

    /// Validate that the token and user list are usable.
    fn validate(&self) -> AppResult<()> {
        if self.bot_token.trim().is_empty() {
            return Err(AppError::Config("bot token is empty".to_owned()));
        }
        if self.allowed_user_ids.is_empty() {
            return Err(AppError::Config(
                "at least one allowed user id is required".to_owned(),
            ));
        }
        let unique: HashSet<u64> = self.allowed_user_ids.iter().copied().collect();
        if unique.len() != self.allowed_user_ids.len() {
            return Err(AppError::Config(
                "allowed user ids contain duplicates".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Parse a comma or semicolon separated list of Telegram user ids.
///
/// # Errors
/// Returns [`AppError::Config`] for any non-numeric entry.
pub fn parse_user_ids(raw: &str) -> AppResult<Vec<u64>> {
    raw.split([',', ';'])
        .map(str::trim)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| {
            chunk
                .parse::<u64>()
                .map_err(|_| AppError::Config(format!("invalid Telegram user id: {chunk}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_id() {
        assert_eq!(parse_user_ids("123").expect("valid"), vec![123]);
    }

    #[test]
    fn parses_mixed_separators() {
        assert_eq!(parse_user_ids("1, 2;3").expect("valid"), vec![1, 2, 3]);
    }

    #[test]
    fn ignores_blank_entries() {
        assert!(parse_user_ids(" , ; ").expect("valid").is_empty());
    }

    #[test]
    fn rejects_non_numeric_id() {
        assert!(parse_user_ids("abc").is_err());
    }

    #[test]
    fn validate_rejects_empty_token() {
        let config = Config {
            bot_token: "   ".to_owned(),
            allowed_user_ids: vec![1],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_no_users() {
        let config = Config {
            bot_token: "123:ABC".to_owned(),
            allowed_user_ids: Vec::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_duplicates() {
        let config = Config {
            bot_token: "123:ABC".to_owned(),
            allowed_user_ids: vec![1, 1],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed() {
        let config = Config {
            bot_token: "123:ABC".to_owned(),
            allowed_user_ids: vec![1, 2],
        };
        assert!(config.validate().is_ok());
        assert!(config.is_allowed(1));
        assert_eq!(config.allowed_user_count(), 2);
    }
}
