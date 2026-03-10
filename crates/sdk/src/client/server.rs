use api::ArtifactApi;
use chrono::{DateTime, TimeDelta, Utc};
use log::{debug, error};
use openapi::models::StatusSchema;
use std::sync::{Arc, RwLock};

#[derive(Default, Debug, Clone)]
pub struct ServerClient(Arc<ServerClientInner>);

#[derive(Default, Debug)]
pub struct ServerClientInner {
    api: ArtifactApi,
    status: RwLock<StatusSchema>,
    time_offset: RwLock<TimeDelta>,
}

impl ServerClient {
    pub(crate) fn new(api: ArtifactApi) -> Self {
        let server = Self(Arc::new(ServerClientInner {
            api,
            status: Default::default(),
            time_offset: Default::default(),
        }));
        server.update_offset();
        server
    }

    pub fn update_status(&self) {
        let Some(status) = self.0.api.server.status() else {
            return;
        };
        *self.0.status.write().unwrap() = *status.data
    }

    pub fn time_offset(&self) -> TimeDelta {
        *self.0.time_offset.read().unwrap()
    }

    fn server_time(&self) -> Option<DateTime<Utc>> {
        let time_str = &self.0.status.read().unwrap().server_time;
        let Ok(time) = DateTime::parse_from_rfc3339(time_str) else {
            return None;
        };
        Some(time.to_utc())
    }

    pub fn update_offset(&self) {
        let now = Utc::now();
        self.update_status();
        let Some(server_time) = self.server_time() else {
            error!("failed to update time offset");
            return;
        };
        *self.0.time_offset.write().unwrap() = now - server_time;
        debug!("system time: {}", now);
        debug!("server time: {}", server_time);
        debug!(
            "time offset: {}s and {}ms",
            self.0.time_offset.read().unwrap().num_seconds(),
            self.0.time_offset.read().unwrap().subsec_nanos() / 1000000
        );
        debug!("synced time: {}", now - *self.0.time_offset.read().unwrap());
    }
}
