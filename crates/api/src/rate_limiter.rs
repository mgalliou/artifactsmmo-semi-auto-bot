use governor::{
    Quota, RateLimiter,
    clock::{Clock, DefaultClock},
    state::InMemoryState,
    state::NotKeyed,
};
use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};
use std::num::NonZeroU32;

const fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("rate limit value must be non-zero")
}

/// Sync poll: block the current thread via `std::thread::sleep` until a
/// token is available.
fn wait_for(lim: &RateLimiter<NotKeyed, InMemoryState, DefaultClock>) {
    let clock = DefaultClock::default();
    loop {
        match lim.check() {
            Ok(()) => return,
            Err(not_until) => {
                std::thread::sleep(not_until.wait_time_from(clock.now()));
            }
        }
    }
}

#[derive(Debug)]
pub struct RateLimiterMiddleware {
    global: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
}

impl RateLimiterMiddleware {
    #[must_use]
    pub fn new() -> Self {
        Self {
            // Server enforces a hard global limit of 10 requests/second.
            // Use 8/s with burst=1 to leave headroom and avoid 429s.
            global: RateLimiter::direct(Quota::per_second(nz(8)).allow_burst(nz(1))),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for RateLimiterMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        wait_for(&self.global);

        next.run(req, extensions).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn nz(v: u32) -> NonZeroU32 {
        NonZeroU32::new(v).unwrap()
    }

    /// Helper that exercises the same polling pattern as `wait_for`.
    fn poll_lim(lim: &RateLimiter<NotKeyed, InMemoryState, DefaultClock>) {
        let clock = DefaultClock::default();
        loop {
            match lim.check() {
                Ok(()) => return,
                Err(not_until) => {
                    std::thread::sleep(not_until.wait_time_from(clock.now()));
                }
            }
        }
    }

    #[test]
    fn enforces_per_second_rate() {
        let lim = RateLimiter::direct(Quota::per_second(nz(8)).allow_burst(nz(1)));
        let start = Instant::now();

        // First call should be near-instant.
        poll_lim(&lim);
        assert!(start.elapsed() < Duration::from_millis(10));

        // Second immediate call must wait.
        poll_lim(&lim);
        let elapsed = start.elapsed();
        // With burst=1, second call must wait at least ~125ms (1/8s).
        assert!(
            elapsed >= Duration::from_millis(120),
            "expected ≥ 120ms, got {elapsed:?}"
        );
    }

    #[test]
    fn sequential_rate_limit() {
        let lim = RateLimiter::direct(Quota::per_second(nz(8)).allow_burst(nz(1)));
        let n = 16;
        let start = Instant::now();
        for _ in 0..n {
            poll_lim(&lim);
        }
        let elapsed = start.elapsed();
        // 16 calls with burst=1: first is instant, 15 more at ~125ms each = ~1875ms.
        let min_expected = Duration::from_millis((n - 1) * 120);
        assert!(
            elapsed >= min_expected,
            "{n} calls took {elapsed:?}, expected ≥ {min_expected:?}"
        );
    }

    #[test]
    fn concurrency_stress_inside_block_on() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let lim = Arc::new(RateLimiter::direct(
            Quota::per_second(nz(8)).allow_burst(nz(1)),
        ));
        let thread_count = 14;
        let calls_per_thread = 3;
        let total_calls = thread_count * calls_per_thread;

        let rt_ref = &rt;
        let start = Instant::now();
        std::thread::scope(|s| {
            for _ in 0..thread_count {
                let lim = lim.clone();
                s.spawn(move || {
                    rt_ref.block_on(async {
                        for _ in 0..calls_per_thread {
                            poll_lim(&lim);
                        }
                    });
                });
            }
        });
        let elapsed = start.elapsed();

        // 42 calls at 8/s with burst=1: first instant, remaining 41 at ~125ms each.
        let min_expected = Duration::from_millis((total_calls - 1) * 120);
        assert!(
            elapsed >= min_expected,
            "{total_calls} concurrent calls inside block_on took {elapsed:?}, expected ≥ {min_expected:?}"
        );
    }
}
