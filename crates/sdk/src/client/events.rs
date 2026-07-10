use crate::{
    Cached,
    entities::{ActiveEvent, Event},
};
use arc_swap::ArcSwap;
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

type FetchData = Box<dyn Fn() -> HashMap<String, Event> + Send + Sync + 'static>;
type FetchActive = Box<dyn Fn() -> Vec<ActiveEvent> + Send + Sync + 'static>;

#[derive(Clone, Default, Deref, CollectionClient)]
#[deref(forward)]
#[element(Event)]
pub struct EventsClient(Arc<EventsClientInner>);

pub struct EventsClientInner {
    directory: Box<str>,
    data: ArcSwap<HashMap<String, Event>>,
    fetch: FetchData,
    fetch_active: FetchActive,
    active: RwLock<Vec<ActiveEvent>>,
    last_refresh: RwLock<DateTime<Utc>>,
}

impl Default for EventsClientInner {
    fn default() -> Self {
        Self {
            directory: ".cache/".into(),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("EventsClient not initialized")),
            fetch_active: Box::new(|| panic!("EventsClient not initialized")),
            active: RwLock::default(),
            last_refresh: RwLock::default(),
        }
    }
}

impl EventsClient {
    #[must_use]
    pub(crate) fn new(path: &str, fetch: FetchData, fetch_active: FetchActive) -> Self {
        Self(Arc::new(EventsClientInner {
            directory: path.into(),
            data: ArcSwap::default(),
            fetch,
            fetch_active,
            active: RwLock::default(),
            last_refresh: RwLock::default(),
        }))
    }

    #[must_use]
    pub fn from_cache(path: &str) -> Self {
        let client = Self::new(
            path,
            Box::new(|| unreachable!("EventsClient::from_cache has no API fallback")),
            Box::new(|| unreachable!("EventsClient::from_cache has no API fallback")),
        );
        client.init();
        client
    }

    pub fn init(&self) {
        let () = thread::scope(|s| {
            // TODO: handle errors
            let _ = s.spawn(|| self.0.data.store(Arc::new(self.fetch())));
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
        *events = (self.fetch_active)();
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

impl Cached<HashMap<String, Event>> for EventsClient {
    const FILE: &str = "events";

    fn directory(&self) -> &str {
        &self.directory
    }

    fn fetch_from_source(&self) -> HashMap<String, Event> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.0.data.store(Arc::new(self.fetch_from_source()));
    }
}
