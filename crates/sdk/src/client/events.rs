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

#[derive(Default, Debug, CollectionClient)]
pub struct EventsClient {
    data: RwLock<HashMap<String, Event>>,
    api: Arc<ArtifactApi>,
    active: RwLock<Vec<ActiveEvent>>,
    last_refresh: RwLock<DateTime<Utc>>,
}

impl EventsClient {
    pub(crate) fn new(api: Arc<ArtifactApi>) -> Self {
        let events = Self {
            data: Default::default(),
            api,
            active: RwLock::new(vec![]),
            last_refresh: RwLock::new(DateTime::<Utc>::MIN_UTC),
        };
        *events.data.write().unwrap() = events.load();
        events.refresh_active();
        events
    }

    pub fn active(&self) -> Vec<ActiveEvent> {
        self.active.read().unwrap().iter().cloned().collect_vec()
    }

    pub fn refresh_active(&self) {
        let now = Utc::now();
        if Utc::now() - self.last_refresh() <= Duration::seconds(30) {
            return;
        }
        // NOTE: keep `events` locked before updating last refresh
        let mut events = self.active.write().unwrap();
        self.update_last_refresh(now);
        if let Ok(new) = self.api.events.get_active() {
            *events = new.into_iter().map(ActiveEvent::new).collect_vec();
            debug!("events refreshed.");
        }
    }

    fn update_last_refresh(&self, now: DateTime<Utc>) {
        self.last_refresh
            .write()
            .expect("`last_refresh` to be writable")
            .clone_from(&now);
    }

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
        *self.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for EventsClient {
    type Entity = Event;
}
