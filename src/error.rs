use thiserror::Error as ThisError;

/// Custom error type.
#[derive(Debug, ThisError)]
pub enum Error {
    /// Error that may occur while giving wrong arguments.
    #[error("Arguments error: `{0}`")]
    ArgsError(String),
    /// Error that may occur while generate log file.
    #[error("Arguments error: `{0}`")]
    LogFileError(#[from] log::SetLoggerError),
    /// Error that may occur during Config operations.
    #[error("Config error: `{0}`")]
    ConfigError(#[from] std::io::Error),
    /// Error that may occur while parsing integers.
    #[error("Failed to parse integer: `{0}`")]
    IntParseError(#[from] std::num::TryFromIntError),
    /// Error that may occur while parsing serde.
    #[error("Failed to parse serde: `{0}`")]
    SerdeParseError(#[from] serde_json::Error),
    /// Error that may occur while receiving messages from the channel.
    #[error("Channel receive error: `{0}`")]
    ChannelReceiveError(#[from] std::sync::mpsc::RecvError),
    /// Error that may occur while sending messages to the channel.
    #[error("Channel send error: `{0}`")]
    ChannelSendError(String),
    /// Error that may occur during config export.
    #[error("Export error: `{0}`")]
    ExportError(String),
}

/// Type alias for the standard [`Result`] type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Error as IoError;

    #[test]
    fn test_error() {
        let message = "your config look very weird!";
        let error = Error::from(IoError::other(message));
        assert_eq!(format!("Config error: `{message}`"), error.to_string());
        assert_eq!(
            format!("\"Config error: `{message}`\""),
            format!("{:?}", error.to_string())
        );
    }
}
