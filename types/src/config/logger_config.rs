use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LoggerConfig {
    // channel size for the asychronous channel for node logging.
    pub channel_size: usize,
    // Use async logging
    pub is_async: bool,
    // The default logging level for slog.
    pub level: String,
}

impl Default for LoggerConfig {
    fn default() -> LoggerConfig {
        LoggerConfig {
            channel_size: 10000,
            is_async: true,
            level: "INFO".to_string(),
        }
    }
}