//! Logging infrastructure for RemoteDesk
//!
//! This module sets up structured logging using the tracing crate.
//! Provides different log levels and formatting options.

use tracing_subscriber::{fmt, EnvFilter};

/// Log level configuration
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    /// Trace level - very verbose
    Trace,
    /// Debug level - detailed information
    Debug,
    /// Info level - general information
    Info,
    /// Warn level - warnings
    Warn,
    /// Error level - errors only
    Error,
}

impl LogLevel {
    /// Converts LogLevel to tracing level filter string
    fn as_filter_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        // Info level by default
        LogLevel::Info
    }
}

/// Initializes the logging system
///
/// Sets up tracing subscriber with the specified log level.
/// Can be overridden by RUST_LOG environment variable.
///
/// # Arguments
///
/// * `level` - The default log level to use
///
/// # Examples
///
/// ```no_run
/// use remote_desk::logging::{init_logging, LogLevel};
///
/// // Initialize with info level
/// init_logging(LogLevel::Info);
///
/// // Or use RUST_LOG environment variable:
/// // RUST_LOG=debug cargo run
/// ```
pub fn init_logging(level: LogLevel) {
    // Create env filter with default level
    let default_filter = format!("remote_desk={}", level.as_filter_str());

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&default_filter));

    // Set up the subscriber
    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();
}

/// Initializes logging with default settings
pub fn init_default_logging() {
    init_logging(LogLevel::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_filter() {
        assert_eq!(LogLevel::Trace.as_filter_str(), "trace");
        assert_eq!(LogLevel::Debug.as_filter_str(), "debug");
        assert_eq!(LogLevel::Info.as_filter_str(), "info");
        assert_eq!(LogLevel::Warn.as_filter_str(), "warn");
        assert_eq!(LogLevel::Error.as_filter_str(), "error");
    }

    #[test]
    fn test_default_log_level() {
        let default = LogLevel::default();
        assert_eq!(default.as_filter_str(), "info");
    }
}
