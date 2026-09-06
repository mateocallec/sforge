use std::thread;
use std::time::Duration;

use crate::error::Result;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub delay: Duration,
}

impl RetryPolicy {
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            delay: Duration::ZERO,
        }
    }
}

/// Retries `attempt` up to `policy.max_retries` additional times on
/// failure, sleeping `policy.delay` between tries. Scoped to connection
/// and transport-level failures — HTTP responses (even error statuses)
/// are always `Ok` from the engine's perspective and are never retried
/// here, matching curl's own `--retry` semantics.
pub fn with_retry<T>(policy: &RetryPolicy, mut attempt: impl FnMut() -> Result<T>) -> Result<T> {
    let mut tries = 0;
    loop {
        match attempt() {
            Ok(value) => return Ok(value),
            Err(_err) if tries < policy.max_retries => {
                tries += 1;
                thread::sleep(policy.delay);
            }
            Err(err) => return Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn succeeds_without_retrying_when_the_first_attempt_works() {
        let calls = AtomicU32::new(0);
        let result = with_retry(&RetryPolicy::none(), || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok::<_, crate::error::Error>(42)
        });
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn retries_up_to_the_configured_limit_then_gives_up() {
        let calls = AtomicU32::new(0);
        let policy = RetryPolicy {
            max_retries: 2,
            delay: Duration::from_millis(1),
        };
        let result = with_retry(&policy, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Err::<(), _>(crate::error::Error::Connect("boom".to_string()))
        });
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn stops_retrying_as_soon_as_an_attempt_succeeds() {
        let calls = AtomicU32::new(0);
        let policy = RetryPolicy {
            max_retries: 5,
            delay: Duration::from_millis(1),
        };
        let result = with_retry(&policy, || {
            let n = calls.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                Err(crate::error::Error::Connect("boom".to_string()))
            } else {
                Ok(())
            }
        });
        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }
}
