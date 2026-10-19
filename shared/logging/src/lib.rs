#![forbid(unsafe_code)]
//! Shared Logging Architecture
//!
//! Provides non-blocking structured logging, rolling file appenders, and
//! configurable formats (pretty, compact, JSON) using the tracing ecosystem.

use std::path::Path;
use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

// Re-export common tracing macros for consumer crates
pub use tracing::{debug, error, info, trace, warn};

/// Logging error definitions
#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("Failed to initialize tracing subscriber: {0}")]
    InitFailed(String),
    #[error("I/O error setting up log directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid log level string '{0}'")]
    InvalidLogLevel(String),
}

/// Output log format style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Compact,
    Json,
}

/// Logging configuration options
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub log_to_stdout: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
            log_to_stdout: true,
        }
    }
}

/// Container retaining background log flusher guards
pub struct LoggingGuard {
    _file_guard: Option<WorkerGuard>,
}

/// Initialize tracing with both console and rolling file output.
pub fn init_file_logging(
    log_dir: &Path,
    file_prefix: &str,
    config: &LoggingConfig,
) -> Result<LoggingGuard, LoggingError> {
    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, file_prefix);
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    let file_layer = fmt::layer()
        .with_writer(non_blocking_appender)
        .with_ansi(false)
        .json();

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);

    if config.log_to_stdout {
        match config.format {
            LogFormat::Pretty => {
                let stdout_layer = fmt::layer().pretty();
                registry
                    .with(stdout_layer)
                    .try_init()
                    .map_err(|e| LoggingError::InitFailed(e.to_string()))?;
            }
            LogFormat::Compact => {
                let stdout_layer = fmt::layer().compact();
                registry
                    .with(stdout_layer)
                    .try_init()
                    .map_err(|e| LoggingError::InitFailed(e.to_string()))?;
            }
            LogFormat::Json => {
                let stdout_layer = fmt::layer().json();
                registry
                    .with(stdout_layer)
                    .try_init()
                    .map_err(|e| LoggingError::InitFailed(e.to_string()))?;
            }
        }
    } else {
        registry
            .try_init()
            .map_err(|e| LoggingError::InitFailed(e.to_string()))?;
    }

    Ok(LoggingGuard {
        _file_guard: Some(guard),
    })
}

/// Parse log level string to LevelFilter
pub fn parse_level_filter(level: &str) -> Result<LevelFilter, LoggingError> {
    match level.to_lowercase().as_str() {
        "error" => Ok(LevelFilter::ERROR),
        "warn" => Ok(LevelFilter::WARN),
        "info" => Ok(LevelFilter::INFO),
        "debug" => Ok(LevelFilter::DEBUG),
        "trace" => Ok(LevelFilter::TRACE),
        "off" => Ok(LevelFilter::OFF),
        other => Err(LoggingError::InvalidLogLevel(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_level_filters() {
        assert_eq!(parse_level_filter("error").unwrap(), LevelFilter::ERROR);
        assert_eq!(parse_level_filter("WARN").unwrap(), LevelFilter::WARN);
        assert_eq!(parse_level_filter("Info").unwrap(), LevelFilter::INFO);
        assert_eq!(parse_level_filter("debug").unwrap(), LevelFilter::DEBUG);
        assert_eq!(parse_level_filter("trace").unwrap(), LevelFilter::TRACE);
        assert_eq!(parse_level_filter("off").unwrap(), LevelFilter::OFF);
    }

    #[test]
    fn test_parse_invalid_level_filter() {
        assert!(parse_level_filter("invalid_level").is_err());
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.format, LogFormat::Pretty);
        assert!(config.log_to_stdout);
    }
}
