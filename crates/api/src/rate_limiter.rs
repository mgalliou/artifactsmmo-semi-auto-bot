use governor::{
    clock::DefaultClock, state::InMemoryState, state::NotKeyed, Quota, RateLimiter,
};
use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};
use std::num::NonZeroU32;

const fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("rate limit value must be non-zero")
}

#[derive(Debug)]
pub struct RateLimiterMiddleware {
    account_per_sec: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    account_per_hour: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    token_per_sec: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    token_per_hour: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    data_per_sec: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    data_per_min: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    data_per_hour: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    action_per_sec: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    action_per_min: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    action_per_hour: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
}

impl RateLimiterMiddleware {
    #[must_use]
    pub fn new() -> Self {
        Self {
            account_per_sec: RateLimiter::direct(Quota::per_second(nz(8)).allow_burst(nz(1))),
            account_per_hour: RateLimiter::direct(Quota::per_hour(nz(300))),
            token_per_sec: RateLimiter::direct(Quota::per_second(nz(8)).allow_burst(nz(1))),
            token_per_hour: RateLimiter::direct(Quota::per_hour(nz(300))),
            data_per_sec: RateLimiter::direct(Quota::per_second(nz(8)).allow_burst(nz(1))),
            data_per_min: RateLimiter::direct(Quota::per_minute(nz(200))),
            data_per_hour: RateLimiter::direct(Quota::per_hour(nz(2000))),
            action_per_sec: RateLimiter::direct(Quota::per_second(nz(8)).allow_burst(nz(1))),
            action_per_min: RateLimiter::direct(Quota::per_minute(nz(100))),
            action_per_hour: RateLimiter::direct(Quota::per_hour(nz(5000))),
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
        let path = req.url().path().to_owned();

        if path.starts_with("/token") {
            self.token_per_sec.until_ready().await;
            self.token_per_hour.until_ready().await;
        } else if path.contains("/action/") {
            self.action_per_sec.until_ready().await;
            self.action_per_min.until_ready().await;
            self.action_per_hour.until_ready().await;
        } else if path.starts_with("/accounts/")
            || path == "/my/change_password"
            || path == "/my/details"
        {
            self.account_per_sec.until_ready().await;
            self.account_per_hour.until_ready().await;
        } else {
            self.data_per_sec.until_ready().await;
            self.data_per_min.until_ready().await;
            self.data_per_hour.until_ready().await;
        }

        next.run(req, extensions).await
    }
}
