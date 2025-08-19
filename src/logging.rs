//! Production-ready structured logging configuration
//!
//! Provides cloud-native logging with:
//! - JSON output for production
//! - Pretty formatting for development
//! - Configurable via environment variables
//! - Automatic context propagation

use crate::config::get_config;
use tracing::Span;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use uuid::Uuid;

/// Initialize the logging system based on configuration
pub fn init_logging() {
    let config = get_config();

    // Use configuration values
    let log_level = &config.logging.level;
    let log_output = &config.logging.output;
    let log_format = &config.logging.format;

    // Build environment filter
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    // Configure output based on config
    match log_output.as_str() {
        "file" => init_file_logging(env_filter, log_format, &config.paths.log_directory),
        "both" => init_combined_logging(env_filter, log_format, &config.paths.log_directory),
        _ => init_console_logging(env_filter, log_format),
    }
}

fn init_console_logging(filter: EnvFilter, format: &str) {
    let subscriber = tracing_subscriber::registry().with(filter);

    match format {
        "json" => {
            subscriber
                .with(
                    fmt::layer()
                        .json()
                        .with_current_span(true)
                        .with_span_list(true)
                        .with_target(true)
                        .with_file(true)
                        .with_line_number(true),
                )
                .init();
        }
        _ => {
            subscriber
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_ansi(true)
                        .with_span_events(FmtSpan::CLOSE)
                        .pretty(),
                )
                .init();
        }
    }
}

fn init_file_logging(filter: EnvFilter, format: &str, log_dir: &std::path::Path) {
    let file_appender = tracing_appender::rolling::daily(log_dir, "claude-usage.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry().with(filter);

    match format {
        "json" => {
            subscriber
                .with(
                    fmt::layer()
                        .json()
                        .with_writer(non_blocking)
                        .with_current_span(true)
                        .with_span_list(true),
                )
                .init();
        }
        _ => {
            subscriber
                .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
                .init();
        }
    }
}

fn init_combined_logging(filter: EnvFilter, format: &str, log_dir: &std::path::Path) {
    let file_appender = tracing_appender::rolling::daily(log_dir, "claude-usage.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry().with(filter);

    match format {
        "json" => {
            subscriber
                .with(fmt::layer().json().with_writer(std::io::stdout))
                .with(fmt::layer().json().with_writer(non_blocking))
                .init();
        }
        _ => {
            subscriber
                .with(fmt::layer().pretty().with_writer(std::io::stdout))
                .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
                .init();
        }
    }
}

/// Create a span with session context
#[macro_export]
macro_rules! span_with_context {
    ($level:expr, $name:expr, $($field:tt)*) => {
        tracing::span!($level, $name, session_id = %$crate::logging::current_session_id(), $($field)*)
    };
}

/// Get current session ID from context
#[allow(dead_code)]
pub fn current_session_id() -> String {
    // In a real implementation, this would use thread-local or async-local storage
    // For now, generate a new one if not in span
    Span::current()
        .field("session_id")
        .map(|f| f.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}
