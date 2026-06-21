use crate::{
    Data, DataEntity, Persist,
    entities::{ActiveEvent, Event},
};
use api::ArtifactApi;
use chrono::{DateTime, Duration, Utc};
use derive_more::Deref;
use itertools::Itertools;
use log::{debug, info};
use sdk_derive::CollectionClient;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockWriteGuard},
    thread,
};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
pub struct EventsClient(Arc<EventsClientInner>);

#[derive(Default, Debug)]
pub struct EventsClientInner {
    api: ArtifactApi,
    data: RwLock<Arc<HashMap<String, Event>>>,
    active: RwLock<Vec<ActiveEvent>>,
    last_refresh: RwLock<DateTime<Utc>>,
}

impl EventsClient {
    pub(crate) fn new(api: ArtifactApi) -> Self {
        Self(
            EventsClientInner {
                api,
                data: RwLock::default(),
                active: RwLock::default(),
                last_refresh: RwLock::default(),
            }
            .into(),
        )
    }

    pub fn init(&self) {
        let () = thread::scope(|s| {
            // TODO: handle errors
            let _ = s.spawn(|| *self.data_mut() = Arc::new(self.load()));
            let _ = s.spawn(|| self.refresh_active());
        });
        info!("Event client initilized");
    }

    #[must_use]
    pub fn active(&self) -> Vec<ActiveEvent> {
        self.active.read().unwrap().iter().cloned().collect_vec()
    }

    fn active_mut(&self) -> RwLockWriteGuard<'_, Vec<ActiveEvent>> {
        self.active.write().unwrap()
    }

    pub fn refresh_active(&self) {
        let mut events = self.active_mut();
        let now = Utc::now();
        // Only refresh active events if they have not been refreshed recently
        if now - self.last_refresh() <= Duration::seconds(30) {
            return;
        }
        self.update_last_refresh(now);
        let Ok(new_schemas) = self.api.events.get_active() else {
            return;
        };
        *events = new_schemas.into_iter().map(ActiveEvent::new).collect_vec();
        debug!("events refreshed.");
    }

    fn update_last_refresh(&self, now: DateTime<Utc>) {
        self.last_refresh
            .write()
            .expect("`last_refresh` to be writable")
            .clone_from(&now);
    }

    #[must_use]
    pub fn last_refresh(&self) -> DateTime<Utc> {
        *self
            .last_refresh
            .read()
            .expect("`last_refresh` to be readable")
    }
}

impl Persist<HashMap<String, Event>> for EventsClient {
    const PATH: &'static str = ".cache/events.json";

    fn load_from_api(&self) -> HashMap<String, Event> {
        self.api
            .events
            .get_all()
            .unwrap()
            .into_iter()
            .map(|event| (event.code.clone(), Event::new(event)))
            .collect()
    }

    fn refresh(&self) {
        *self.data_mut() = Arc::new(self.load_from_api());
    }
}

impl DataEntity for EventsClient {
    type Entity = Event;
}
