use crate::API;
use artifactsmmo_openapi::models::StatusResponseSchema;
use chrono::{DateTime, TimeDelta, Utc};
use log::{debug, error};
use std::sync::{LazyLock, RwLock};

pub static SERVER: LazyLock<Server> = LazyLock::new(Server::new);

#[derive(Default)]
pub struct Server {
    pub server_offset: RwLock<TimeDelta>,
}

impl Server {
    fn new() -> Self {
        let server = Self {
            server_offset: RwLock::new(TimeDelta::default()),
        };
        server.update_offset();
        server
    }

    pub fn status(&self) -> Option<StatusResponseSchema> {
        API.server.status()
    }

    pub fn time(&self) -> Option<DateTime<Utc>> {
        let status = self.status()?;
        let Ok(time) = DateTime::parse_from_rfc3339(&status.data.server_time) else {
            return None;
        };
        Some(time.to_utc())
    }

    pub fn update_offset(&self) {
        let now = Utc::now();
        let Some(server_time) = self.time() else {
            error!("failed to update time offset");
            return;
        };
        let round_trip = now - Utc::now();
        *self.server_offset.write().unwrap() = now - server_time + (round_trip / 2);
        debug!("system time: {}", now);
        debug!("server time: {}", server_time);
        debug!(
            "time offset: {}s and {}ms",
            self.server_offset.read().unwrap().num_seconds(),
            self.server_offset.read().unwrap().subsec_nanos() / 1000000
        );
        debug!("synced time: {}", now - *self.server_offset.read().unwrap());
    }
}
