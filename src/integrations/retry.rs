//! Retry with exponential backoff for external API calls
//!
//! Provides resilient API calls that automatically retry on transient failures
//! with exponential backoff and jitter.

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (not including the initial attempt)
    pub max_retries: u32,

    /// Initial backoff duration
    pub initial_backoff: Duration,

    /// Maximum backoff duration
    pub max_backoff: Duration,

    /// Backoff multiplier (typically 2.0 for exponential backoff)
    pub multiplier: f64,

    /// Add random jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a config for rate-limited APIs (longer initial backoff)
    pub fn for_rate_limited() -> Self {
        Self {
            max_retries: 5,
            initial_backoff: Duration::from_secs(5),
            max_backoff: Duration::from_secs(300), // 5 minutes
            multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a config for quick retries (short backoff)
    pub fn quick() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
            multiplier: 2.0,
            jitter: true,
        }
    }

    /// Calculate backoff duration for a given attempt
    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        let base = self.initial_backoff.as_secs_f64() * self.multiplier.powi(attempt as i32);
        let capped = base.min(self.max_backoff.as_secs_f64());

        let final_duration = if self.jitter {
            // Add 0-25% jitter
            let jitter_factor = 1.0 + (rand_jitter() * 0.25);
            capped * jitter_factor
        } else {
            capped
        };

        Duration::from_secs_f64(final_duration)
    }
}

/// Simple pseudo-random jitter (0.0 to 1.0) without external dependency
fn rand_jitter() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos % 1000) as f64 / 1000.0
}

/// Retry classification for errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// Retry the operation
    Retry,
    /// Retry after a specific duration (e.g., from Retry-After header)
    RetryAfter(Duration),
    /// Don't retry, the error is permanent
    NoRetry,
}

/// Trait for errors that can indicate whether to retry
pub trait RetryableError {
    /// Determine if this error should be retried
    fn retry_decision(&self) -> RetryDecision;
}

/// Execute an async operation with retry logic
///
/// # Arguments
/// * `config` - Retry configuration
/// * `operation_name` - Name for logging purposes
/// * `operation` - The async operation to execute
///
/// # Returns
/// The result of the operation, or the last error if all retries failed
pub async fn with_retry<F, Fut, T, E>(
    config: &RetryConfig,
    operation_name: &str,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: RetryableError + std::fmt::Display,
{
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let decision = e.retry_decision();

                match decision {
                    RetryDecision::NoRetry => {
                        debug!(
                            operation = operation_name,
                            attempt = attempt,
                            "Operation failed with non-retryable error: {}",
                            e
                        );
                        return Err(e);
                    }
                    RetryDecision::Retry | RetryDecision::RetryAfter(_) => {
                        if attempt >= config.max_retries {
                            warn!(
                                operation = operation_name,
                                attempts = attempt + 1,
                                "Operation failed after {} attempts: {}",
                                attempt + 1,
                                e
                            );
                            return Err(e);
                        }

                        let backoff = match decision {
                            RetryDecision::RetryAfter(d) => d.min(config.max_backoff),
                            _ => config.backoff_duration(attempt),
                        };

                        warn!(
                            operation = operation_name,
                            attempt = attempt + 1,
                            max_attempts = config.max_retries + 1,
                            backoff_secs = backoff.as_secs_f64(),
                            "Retrying after error: {}",
                            e
                        );

                        sleep(backoff).await;
                        attempt += 1;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_duration() {
        let config = RetryConfig {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            multiplier: 2.0,
            jitter: false,
            ..Default::default()
        };

        // Without jitter, should be exactly: 1, 2, 4, 8, 16, 32, 60 (capped)
        assert_eq!(config.backoff_duration(0), Duration::from_secs(1));
        assert_eq!(config.backoff_duration(1), Duration::from_secs(2));
        assert_eq!(config.backoff_duration(2), Duration::from_secs(4));
        assert_eq!(config.backoff_duration(3), Duration::from_secs(8));
        assert_eq!(config.backoff_duration(6), Duration::from_secs(60)); // Capped
    }

    #[test]
    fn test_backoff_with_jitter() {
        let config = RetryConfig {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            multiplier: 2.0,
            jitter: true,
            ..Default::default()
        };

        // With jitter, should be between 1.0 and 1.25 seconds for attempt 0
        let backoff = config.backoff_duration(0);
        assert!(backoff >= Duration::from_secs(1));
        assert!(backoff <= Duration::from_millis(1250));
    }

    #[derive(Debug)]
    struct TestError {
        retryable: bool,
    }

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestError(retryable={})", self.retryable)
        }
    }

    impl RetryableError for TestError {
        fn retry_decision(&self) -> RetryDecision {
            if self.retryable {
                RetryDecision::Retry
            } else {
                RetryDecision::NoRetry
            }
        }
    }

    #[tokio::test]
    async fn test_retry_succeeds_eventually() {
        let config = RetryConfig::quick();
        let mut attempts = 0;

        let result: Result<&str, TestError> = with_retry(&config, "test", || {
            attempts += 1;
            async move {
                if attempts < 3 {
                    Err(TestError { retryable: true })
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_gives_up() {
        let config = RetryConfig {
            max_retries: 2,
            initial_backoff: Duration::from_millis(1),
            ..Default::default()
        };
        let mut attempts = 0;

        let result: Result<&str, TestError> = with_retry(&config, "test", || {
            attempts += 1;
            async move { Err(TestError { retryable: true }) }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 3); // Initial + 2 retries
    }

    #[tokio::test]
    async fn test_no_retry_on_permanent_error() {
        let config = RetryConfig::quick();
        let mut attempts = 0;

        let result: Result<&str, TestError> = with_retry(&config, "test", || {
            attempts += 1;
            async move { Err(TestError { retryable: false }) }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 1); // No retries
    }
}
