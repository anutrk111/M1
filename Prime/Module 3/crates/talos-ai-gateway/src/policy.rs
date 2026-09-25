//! Resilience primitives: backoff with jitter, circuit breaker, token bucket.
//! All timing uses `tokio::time` so tests can run on a paused clock.

use crate::config::{BreakerConfig, RateLimitConfig, RetryConfig};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::Instant;

/// Exponential backoff `base * 2^attempt`, capped at `max`, scaled by a jitter
/// factor in `[0.5, 1.0]`.
pub struct Backoff {
    base_ms: u64,
    max_ms: u64,
    state: AtomicU64,
}

impl Backoff {
    pub fn new(cfg: &RetryConfig) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
        Self {
            base_ms: cfg.base_delay_ms,
            max_ms: cfg.max_delay_ms,
            state: AtomicU64::new(seed),
        }
    }

    fn next_unit(&self) -> f64 {
        // xorshift64*: jitter only, not security-sensitive.
        let mut x = self.state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state.store(x, Ordering::Relaxed);
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn delay(&self, attempt: u32) -> Duration {
        let exp = self
            .base_ms
            .saturating_mul(1u64 << attempt.min(20))
            .min(self.max_ms);
        let factor = 0.5 + 0.5 * self.next_unit();
        Duration::from_millis((exp as f64 * factor) as u64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

struct BreakerInner {
    state: BreakerState,
    consecutive_failures: u32,
    opened_at: Option<Instant>,
    half_open_in_flight: u32,
}

/// Closed → Open after `failure_threshold` consecutive transient failures;
/// Open → HalfOpen after `open_ms`; a half-open success closes, a failure reopens.
pub struct CircuitBreaker {
    cfg: BreakerConfig,
    inner: Mutex<BreakerInner>,
}

impl CircuitBreaker {
    pub fn new(cfg: BreakerConfig) -> Self {
        Self {
            cfg,
            inner: Mutex::new(BreakerInner {
                state: BreakerState::Closed,
                consecutive_failures: 0,
                opened_at: None,
                half_open_in_flight: 0,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BreakerInner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Returns `true` when a call may proceed. Every `true` must be followed by
    /// exactly one of `on_success`, `on_failure`, or `on_neutral`.
    pub fn try_acquire(&self) -> bool {
        let mut s = self.lock();
        if s.state == BreakerState::Open {
            let elapsed = s.opened_at.map(|t| t.elapsed()).unwrap_or_default();
            if elapsed >= Duration::from_millis(self.cfg.open_ms) {
                s.state = BreakerState::HalfOpen;
                s.half_open_in_flight = 0;
            } else {
                return false;
            }
        }
        if s.state == BreakerState::HalfOpen {
            if s.half_open_in_flight >= self.cfg.half_open_max_calls.max(1) {
                return false;
            }
            s.half_open_in_flight += 1;
        }
        true
    }

    pub fn on_success(&self) {
        let mut s = self.lock();
        s.state = BreakerState::Closed;
        s.consecutive_failures = 0;
        s.opened_at = None;
        s.half_open_in_flight = 0;
    }

    pub fn on_failure(&self) {
        let mut s = self.lock();
        match s.state {
            BreakerState::HalfOpen => {
                s.state = BreakerState::Open;
                s.opened_at = Some(Instant::now());
                s.half_open_in_flight = 0;
            }
            BreakerState::Closed => {
                s.consecutive_failures += 1;
                if s.consecutive_failures >= self.cfg.failure_threshold.max(1) {
                    s.state = BreakerState::Open;
                    s.opened_at = Some(Instant::now());
                }
            }
            BreakerState::Open => {}
        }
    }

    /// Outcome says nothing about provider health (e.g. caller `Validation`).
    pub fn on_neutral(&self) {
        let mut s = self.lock();
        if s.state == BreakerState::HalfOpen {
            s.half_open_in_flight = s.half_open_in_flight.saturating_sub(1);
        }
    }

    pub fn state(&self) -> BreakerState {
        self.lock().state
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.lock().consecutive_failures
    }
}

struct BucketInner {
    tokens: f64,
    last: Instant,
}

/// Token bucket. `acquire` waits for a token; callers bound the wait with the
/// operation timeout.
pub struct TokenBucket {
    rate: f64,
    capacity: f64,
    inner: Mutex<BucketInner>,
}

impl TokenBucket {
    pub fn new(cfg: &RateLimitConfig) -> Self {
        let capacity = f64::from(cfg.burst.max(1));
        Self {
            rate: cfg.requests_per_second,
            capacity,
            inner: Mutex::new(BucketInner {
                tokens: capacity,
                last: Instant::now(),
            }),
        }
    }

    pub fn is_unlimited(&self) -> bool {
        self.rate <= 0.0 || !self.rate.is_finite()
    }

    pub async fn acquire(&self) {
        if self.is_unlimited() {
            return;
        }
        loop {
            let wait = {
                let mut b = self.inner.lock().unwrap_or_else(|p| p.into_inner());
                let now = Instant::now();
                let refill = now.duration_since(b.last).as_secs_f64() * self.rate;
                b.tokens = (b.tokens + refill).min(self.capacity);
                b.last = now;
                if b.tokens >= 1.0 {
                    b.tokens -= 1.0;
                    return;
                }
                Duration::from_secs_f64((1.0 - b.tokens) / self.rate)
            };
            tokio::time::sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaker(threshold: u32) -> CircuitBreaker {
        CircuitBreaker::new(BreakerConfig {
            failure_threshold: threshold,
            open_ms: 1_000,
            half_open_max_calls: 1,
        })
    }

    #[tokio::test(start_paused = true)]
    async fn breaker_opens_half_opens_and_closes() {
        let b = breaker(2);
        assert!(b.try_acquire());
        b.on_failure();
        assert_eq!(b.state(), BreakerState::Closed);
        assert!(b.try_acquire());
        b.on_failure();
        assert_eq!(b.state(), BreakerState::Open);
        assert!(!b.try_acquire());

        tokio::time::advance(Duration::from_millis(1_001)).await;
        assert!(b.try_acquire());
        assert_eq!(b.state(), BreakerState::HalfOpen);
        assert!(!b.try_acquire(), "only one half-open probe");
        b.on_success();
        assert_eq!(b.state(), BreakerState::Closed);
        assert!(b.try_acquire());
    }

    #[tokio::test(start_paused = true)]
    async fn half_open_failure_reopens() {
        let b = breaker(1);
        assert!(b.try_acquire());
        b.on_failure();
        tokio::time::advance(Duration::from_millis(1_001)).await;
        assert!(b.try_acquire());
        b.on_failure();
        assert_eq!(b.state(), BreakerState::Open);
        assert!(!b.try_acquire());
    }

    #[tokio::test(start_paused = true)]
    async fn success_resets_consecutive_failures() {
        let b = breaker(2);
        b.on_failure();
        b.on_success();
        b.on_failure();
        assert_eq!(b.state(), BreakerState::Closed);
        assert_eq!(b.consecutive_failures(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn token_bucket_waits_when_empty() {
        let bucket = TokenBucket::new(&RateLimitConfig {
            requests_per_second: 2.0,
            burst: 1,
        });
        let start = Instant::now();
        bucket.acquire().await;
        bucket.acquire().await;
        assert!(start.elapsed() >= Duration::from_millis(499));
    }

    #[test]
    fn backoff_is_bounded_and_jittered() {
        let b = Backoff::new(&RetryConfig {
            max_retries: 5,
            base_delay_ms: 100,
            max_delay_ms: 1_000,
        });
        for attempt in 0..10 {
            let d = b.delay(attempt).as_millis() as u64;
            let cap = (100u64 << attempt.min(20)).min(1_000);
            assert!(d <= cap && d >= cap / 2, "attempt {attempt}: {d} vs {cap}");
        }
    }
}
