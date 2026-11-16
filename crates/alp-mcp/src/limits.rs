//! Limits applied at the MCP boundary: payload size, call timeout, and a
//! token-bucket rate limit per client.
//!
//! These exist so one attached agent cannot stall or flood the host. The limits
//! are deliberately generous defaults overridable from the command line, and
//! every rejection names the limit and how to recover, because a silent
//! throttle reads to an agent as a broken tool.
//!
//! The rate limiter is in-process state owned by the server. `alp-mcp` serves
//! one client per process, so "per client" means per process today; the limiter
//! is keyed by a caller-supplied label so it stays correct if the server later
//! multiplexes sessions.

use std::time::{Duration, Instant};

use thiserror::Error;

/// Rejection raised when a request exceeds a boundary limit.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LimitError {
    /// The JSON request line is larger than the configured cap.
    #[error(
        "request payload is {actual} bytes, over the {limit}-byte limit; split the call or send smaller arguments"
    )]
    PayloadTooLarge {
        /// Received size in bytes.
        actual: usize,
        /// Configured cap in bytes.
        limit: usize,
    },

    /// A single tool call ran longer than the configured budget.
    ///
    /// This is deliberately not raised as a tool error: the call ran to
    /// completion and its result is real, so failing it would discard work the
    /// user asked for. It is reported as a note on the result instead, and the
    /// variant exists so callers of this module can distinguish the case.
    #[error(
        "tool call exceeded the {limit_ms} ms time limit; reduce the work (smaller range, shorter text) or raise --call-timeout-ms"
    )]
    Timeout {
        /// Configured budget in milliseconds.
        limit_ms: u64,
    },

    /// The caller exhausted its request budget.
    #[error(
        "rate limit: {used} requests in the last {window_secs}s exceeds the budget of {budget}; wait before retrying"
    )]
    RateLimited {
        /// Requests counted inside the window.
        used: u32,
        /// Allowed requests per window.
        budget: u32,
        /// Window length in seconds.
        window_secs: u64,
    },
}

/// Boundary limits for one MCP server process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum bytes in one JSON-RPC request line.
    pub max_payload_bytes: usize,
    /// Maximum wall-clock time for one tool call.
    pub call_timeout: Duration,
    /// Requests allowed per window, per client.
    pub requests_per_window: u32,
    /// Length of the rate-limit window.
    pub window: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_payload_bytes: 1024 * 1024,
            call_timeout: Duration::from_secs(120),
            requests_per_window: 600,
            window: Duration::from_secs(60),
        }
    }
}

impl Limits {
    /// Reject a request line that is over the payload cap.
    pub fn check_payload(&self, bytes: usize) -> Result<(), LimitError> {
        if bytes > self.max_payload_bytes {
            return Err(LimitError::PayloadTooLarge {
                actual: bytes,
                limit: self.max_payload_bytes,
            });
        }
        Ok(())
    }

    /// Whether a completed call exceeded the advisory time budget.
    pub fn check_elapsed(&self, elapsed: std::time::Duration) -> Result<(), LimitError> {
        if elapsed > self.call_timeout {
            return Err(LimitError::Timeout {
                limit_ms: self.call_timeout.as_millis() as u64,
            });
        }
        Ok(())
    }
}

/// Sliding-window token bucket for one client label.
#[derive(Debug)]
pub struct RateLimiter {
    limits: Limits,
    /// Request timestamps inside the current window, oldest first.
    recent: Vec<Instant>,
}

impl RateLimiter {
    /// Build a limiter for the given limits.
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            recent: Vec::new(),
        }
    }

    /// Count one request at `now`, or reject it.
    ///
    /// Timestamps older than the window are dropped first, so the budget
    /// refills continuously instead of resetting on a fixed boundary.
    pub fn admit(&mut self, now: Instant) -> Result<(), LimitError> {
        let window = self.limits.window;
        self.recent
            .retain(|stamp| now.duration_since(*stamp) < window);
        if self.recent.len() >= self.limits.requests_per_window as usize {
            return Err(LimitError::RateLimited {
                used: self.recent.len() as u32,
                budget: self.limits.requests_per_window,
                window_secs: window.as_secs(),
            });
        }
        self.recent.push(now);
        Ok(())
    }

    /// Requests currently counted inside the window.
    ///
    /// Counts stamps as they stand: call [`RateLimiter::remaining`] first if
    /// you need a value pruned to the live window.
    #[must_use]
    pub fn used(&self) -> usize {
        self.recent.len()
    }

    /// Remaining budget in the current window at `now`.
    ///
    /// Prunes aged-out stamps first, so the value is the budget a caller can
    /// actually spend right now rather than a counter since process start.
    #[must_use]
    pub fn remaining(&mut self, now: Instant) -> usize {
        let window = self.limits.window;
        self.recent
            .retain(|stamp| now.duration_since(*stamp) < window);
        (self.limits.requests_per_window as usize).saturating_sub(self.recent.len())
    }
}

/// How long a completed call really took.
///
/// The timeout is advisory: the call is executed synchronously on the server
/// thread (the session is a single-writer `Mutex`), so a wedged call cannot be
/// interrupted. What this reports honestly is the measured duration, so the
/// caller sees the overrun instead of receiving a silent success.
#[derive(Debug)]
pub struct TimedOutcome<T> {
    /// The produced value.
    pub value: T,
    /// How long the call took.
    pub elapsed: Duration,
}

impl<T> TimedOutcome<T> {
    /// Record a completed call and its measured duration.
    #[must_use]
    pub fn record(value: T, elapsed: Duration) -> Self {
        Self { value, elapsed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_cap_rejects_oversize_and_names_the_budget() {
        let limits = Limits {
            max_payload_bytes: 8,
            ..Limits::default()
        };
        assert!(limits.check_payload(8).is_ok());
        let error = limits.check_payload(9).expect_err("over cap");
        assert_eq!(
            error,
            LimitError::PayloadTooLarge {
                actual: 9,
                limit: 8
            }
        );
        assert!(error.to_string().contains("8-byte limit"));
    }

    #[test]
    fn the_budget_refills_as_the_window_slides() {
        let limits = Limits {
            requests_per_window: 2,
            window: Duration::from_secs(10),
            ..Limits::default()
        };
        let mut limiter = RateLimiter::new(limits);
        let start = Instant::now();

        assert!(limiter.admit(start).is_ok());
        assert!(limiter.admit(start + Duration::from_secs(1)).is_ok());
        let rejected = limiter
            .admit(start + Duration::from_secs(2))
            .expect_err("third request in the window");
        assert_eq!(
            rejected,
            LimitError::RateLimited {
                used: 2,
                budget: 2,
                window_secs: 10
            }
        );

        // After the first two stamps age out, the budget is available again.
        assert!(limiter.admit(start + Duration::from_secs(11)).is_ok());
    }

    #[test]
    fn timeout_reporting_is_honest_about_what_happened() {
        let limits = Limits {
            call_timeout: Duration::from_secs(1),
            ..Limits::default()
        };

        let fast = TimedOutcome::record("done", Duration::from_millis(5));
        assert!(limits.check_elapsed(fast.elapsed).is_ok());

        let slow = TimedOutcome::record("done", Duration::from_secs(2));
        let error = limits
            .check_elapsed(slow.elapsed)
            .expect_err("a call slower than the budget is over budget");
        assert_eq!(error, LimitError::Timeout { limit_ms: 1000 });
        assert_eq!(
            slow.value, "done",
            "the real result is still returned; only the overrun is reported"
        );
    }

    #[test]
    fn remaining_budget_shrinks_and_recovers() {
        let limits = Limits {
            requests_per_window: 3,
            window: Duration::from_secs(10),
            ..Limits::default()
        };
        let mut limiter = RateLimiter::new(limits);
        let start = Instant::now();
        assert_eq!(limiter.remaining(start), 3);
        limiter.admit(start).expect("first");
        assert_eq!(limiter.remaining(start), 2);
        assert_eq!(limiter.used(), 1);
        assert_eq!(limiter.remaining(start + Duration::from_secs(11)), 3);
    }
}
