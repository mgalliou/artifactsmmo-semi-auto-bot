use api::ArtifactApi;
use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use derive_more::Deref;
use log::{debug, info};
use openapi::models::StatusSchema;
use std::sync::{Arc, RwLock};

#[derive(Default, Debug, Clone, Deref)]
#[deref(forward)]
pub struct ServerClient(Arc<ServerClientInner>);

#[derive(Default, Debug)]
pub struct ServerClientInner {
    api: ArtifactApi,
    status: RwLock<StatusSchema>,
    time_offset: RwLock<TimeDelta>,
}

impl ServerClient {
    pub(crate) fn new(api: ArtifactApi) -> Self {
        Self(Arc::new(ServerClientInner {
            api,
            status: RwLock::default(),
            time_offset: RwLock::default(),
        }))
    }

    pub fn init(&self) {
        self.update_offset();
        info!("Server client initilized");
    }

    pub fn update_status(&self) {
        let Some(status) = self.api.server.status() else {
            return;
        };
        *self.status.write().unwrap() = *status.data;
    }

    #[must_use]
    pub fn time_offset(&self) -> TimeDelta {
        *self.time_offset.read().unwrap()
    }

    fn server_time(&self) -> DateTime<FixedOffset> {
        self.status.read().unwrap().server_time
    }

    pub fn update_offset(&self) {
        let now = Utc::now().fixed_offset();
        self.update_status();
        *self.time_offset.write().unwrap() = now - self.server_time();
        debug!("system time: {now}");
        debug!("server time: {}", self.server_time());
        let offset = self.time_offset();
        debug!(
            "server time offset: {}s and {}ms",
            offset.num_seconds(),
            offset / 1_000_000
        );
        debug!("synced time: {}", now - offset);
    }
}
