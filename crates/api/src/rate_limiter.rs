use governor::{
    Quota, RateLimiter,
    clock::{Clock, DefaultClock},
    state::InMemoryState,
    state::NotKeyed,
};
use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};
use std::{num::NonZeroU32, thread::sleep};

const fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("rate limit value must be non-zero")
}

/// block the current thread via `std::thread::sleep` until a
/// token is available.
fn wait_for(limiter: &RateLimiter<NotKeyed, InMemoryState, DefaultClock>) {
    loop {
        match limiter.check() {
            Ok(()) => return,
            Err(not_until) => {
                sleep(not_until.wait_time_from(limiter.clock().now()));
            }
        }
    }
}

#[derive(Debug)]
pub struct RateLimiterMiddleware {
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
}

impl RateLimiterMiddleware {
    #[must_use]
    pub fn new() -> Self {
        Self {
            // Server enforces a hard global limit of 10 requests/second.
            limiter: RateLimiter::direct(Quota::per_second(nz(10))),
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
        wait_for(&self.limiter);

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

    #[test]
    fn enforces_per_second_rate() {
        let lim = RateLimiter::direct(Quota::per_second(nz(8)).allow_burst(nz(1)));
        let start = Instant::now();

        // First call should be near-instant.
        wait_for(&lim);
        assert!(start.elapsed() < Duration::from_millis(10));

        // Second immediate call must wait.
        wait_for(&lim);
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
            wait_for(&lim);
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
                            wait_for(&lim);
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
