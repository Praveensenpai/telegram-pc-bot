use thiserror::Error;

/// Centralized error type for the whole application.
#[derive(Debug, Error)]
pub enum AppError {
    /// A required configuration value is missing or malformed.
    #[error("configuration error: {0}")]
    Config(String),

    /// The Telegram API returned an error.
    #[error("telegram error: {0}")]
    Telegram(#[from] teloxide::RequestError),

    /// Interactive setup input failed.
    #[error("input error: {0}")]
    Input(String),

    /// A scheduled-task (auto-start) operation failed.
    #[error("autostart error: {0}")]
    Autostart(String),

    /// An operating-system level failure occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<dialoguer::Error> for AppError {
    fn from(error: dialoguer::Error) -> Self {
        match error {
            dialoguer::Error::IO(io) => Self::Io(io),
        }
    }
}

/// Convenience alias used across the crate.
pub type AppResult<T> = Result<T, AppError>;
