use crate::{
    DataEntity, Persist,
    entities::{ActiveEvent, Event},
};
use api::ArtifactApi;
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use log::debug;
use sdk_derive::CollectionClient;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, CollectionClient)]
pub struct EventsClient(Arc<EventsClientInner>);

#[derive(Default, Debug)]
pub struct EventsClientInner {
    data: RwLock<HashMap<String, Event>>,
    api: Arc<ArtifactApi>,
    active: RwLock<Vec<ActiveEvent>>,
    last_refresh: RwLock<DateTime<Utc>>,
}

impl EventsClient {
    pub(crate) fn new(api: Arc<ArtifactApi>) -> Self {
        let events = Self(Arc::new(EventsClientInner {
            api,
            data: Default::default(),
            active: Default::default(),
            last_refresh: Default::default(),
        }));
        *events.0.data.write().unwrap() = events.load();
        events.refresh_active();
        events
    }

    pub fn active(&self) -> Vec<ActiveEvent> {
        self.0.active.read().unwrap().iter().cloned().collect_vec()
    }

    pub fn refresh_active(&self) {
        let now = Utc::now();
        if Utc::now() - self.last_refresh() <= Duration::seconds(30) {
            return;
        }
        // NOTE: keep `events` locked before updating last refresh
        let mut events = self.0.active.write().unwrap();
        self.update_last_refresh(now);
        if let Ok(new) = self.0.api.events.get_active() {
            *events = new.into_iter().map(ActiveEvent::new).collect_vec();
            debug!("events refreshed.");
        }
    }

    fn update_last_refresh(&self, now: DateTime<Utc>) {
        self.0
            .last_refresh
            .write()
            .expect("`last_refresh` to be writable")
            .clone_from(&now);
    }

    pub fn last_refresh(&self) -> DateTime<Utc> {
        *self
            .0
            .last_refresh
            .read()
            .expect("`last_refresh` to be readable")
    }
}

impl Persist<HashMap<String, Event>> for EventsClient {
    const PATH: &'static str = ".cache/events.json";

    fn load_from_api(&self) -> HashMap<String, Event> {
        self.0
            .api
            .events
            .get_all()
            .unwrap()
            .into_iter()
            .map(|event| (event.code.clone(), Event::new(event)))
            .collect()
    }

    fn refresh(&self) {
        *self.0.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for EventsClient {
    type Entity = Event;
}
