use api::ArtifactApi;
use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use derive_more::Deref;
use log::{debug, info};
use openapi::models::StatusSchema;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default, Deref)]
#[deref(forward)]
pub struct ServerClient(Arc<ServerClientInner>);

#[derive(Default)]
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

    fn server_time(&self) -> DateTime<FixedOffset> {
        self.status.read().unwrap().server_time
    }

    #[must_use]
    fn time_offset(&self) -> TimeDelta {
        *self.time_offset.read().unwrap()
    }

    #[must_use]
    pub fn synced_time(&self) -> DateTime<Utc> {
        Utc::now() + self.time_offset()
    }

    pub fn update_offset(&self) {
        let send = Utc::now().fixed_offset();
        self.update_status();
        let recv = Utc::now().fixed_offset();
        let rtt = recv - send;
        *self.time_offset.write().unwrap() = self.server_time() + (rtt / 2) - recv;
        debug!("send sys time: {send}");
        debug!("server time  : {}", self.server_time());
        debug!("recv sys tim : {recv}");
        debug!(
            "round trip time : {}.{}s",
            rtt.num_seconds(),
            rtt.subsec_millis()
        );
        let offset = self.time_offset();
        debug!(
            "server time offset: {}.{}s",
            offset.num_seconds(),
            offset.subsec_millis()
        );
        debug!("synced time: {}", recv + offset);
    }
}
