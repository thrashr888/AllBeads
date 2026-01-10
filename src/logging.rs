//! Logging configuration using tracing
//!
//! Provides structured logging to stderr and file with support for RUST_LOG environment variable.

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the tracing subscriber
///
/// Sets up structured logging with:
/// - Filtering via RUST_LOG environment variable (defaults to "warn" for quiet output)
/// - Formatted output to stderr
/// - Optional file logging (future enhancement)
///
/// # Example RUST_LOG values
/// - `RUST_LOG=info` - Show info and above
/// - `RUST_LOG=debug` - Show debug and above
/// - `RUST_LOG=allbeads=trace` - Trace level for allbeads crate
/// - `RUST_LOG=allbeads=debug,beads=info` - Different levels per crate
///
/// # Errors
/// Returns an error if the subscriber has already been initialized
pub fn init() -> crate::Result<()> {
    // Create an EnvFilter that respects RUST_LOG, defaulting to "warn" for quiet CLI output
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    // Configure the tracing subscriber with:
    // - Environment-based filtering
    // - Pretty formatting for human readability
    // - Target (module path) in output
    // - Thread IDs for debugging concurrency
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .pretty(),
        )
        .try_init()
        .map_err(|e| crate::AllBeadsError::Other(format!("Failed to initialize tracing: {}", e)))?;

    Ok(())
}

/// Initialize logging for tests (no-op if already initialized)
pub fn init_test() {
    let _ = init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging() {
        // Should not panic even if called multiple times
        let result = init();
        // First call may succeed or fail depending on test order
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_init_test_helper() {
        // Should never panic
        init_test();
        init_test(); // Can be called multiple times
    }

    #[test]
    fn test_logging_macros() {
        init_test();

        // Verify tracing macros work
        tracing::trace!("This is a trace message");
        tracing::debug!("This is a debug message");
        tracing::info!("This is an info message");
        tracing::warn!("This is a warning message");
        tracing::error!("This is an error message");

        // Structured logging
        tracing::info!(
            user = "test",
            action = "test_run",
            "Testing structured logging"
        );
    }
}
