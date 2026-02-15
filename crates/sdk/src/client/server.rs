use api::ArtifactApi;
use chrono::{DateTime, TimeDelta, Utc};
use log::{debug, error};
use openapi::models::StatusResponseSchema;
use std::sync::{Arc, RwLock};

#[derive(Default, Debug)]
pub struct ServerClient {
    api: Arc<ArtifactApi>,
    pub server_offset: RwLock<TimeDelta>,
}

impl ServerClient {
    pub(crate) fn new(api: Arc<ArtifactApi>) -> Self {
        let server = Self {
            api,
            server_offset: RwLock::new(TimeDelta::default()),
        };
        server.update_offset();
        server
    }

    pub fn status(&self) -> Option<StatusResponseSchema> {
        self.api.server.status()
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
        *self.server_offset.write().unwrap() = now - server_time;
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
