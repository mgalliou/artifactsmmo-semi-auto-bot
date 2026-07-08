use crate::{
    Cached, CanProvideXp, CollectionClient, DropsItems, Level,
    client::events::EventsClient,
    entities::{EventSchemaExt, Monster},
};
use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Default, Deref, CollectionClient)]
#[deref(forward)]
#[element(Monster)]
pub struct MonstersClient(Arc<MonstersClientInner>);

pub struct MonstersClientInner {
    directory: Box<str>,
    data: ArcSwap<HashMap<String, Monster>>,
    fetch: Box<dyn Fn() -> HashMap<String, Monster> + Send + Sync>,
    events: EventsClient,
}

impl Default for MonstersClientInner {
    fn default() -> Self {
        Self {
            directory: ".cache".into(),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("MonstersClient not initialized")),
            events: EventsClient::default(),
        }
    }
}

impl MonstersClient {
    pub(crate) fn new(
        path: &str,
        fetch: Box<dyn Fn() -> HashMap<String, Monster> + Send + Sync>,
        events: EventsClient,
    ) -> Self {
        Self(
            MonstersClientInner {
                directory: path.into(),
                fetch,
                data: ArcSwap::default(),
                events,
            }
            .into(),
        )
    }

    #[must_use]
    pub fn from_cache(path: &str) -> Self {
        let client = Self::new(
            path,
            Box::new(|| unreachable!("MonstersClient::from_cache has no API fallback")),
            EventsClient::default(),
        );
        client.init();
        client
    }

    pub fn init(&self) {
        self.data.store(Arc::new(self.fetch()));
        info!("Monster client initilized");
    }

    #[must_use]
    pub fn dropping(&self, item_code: &str) -> Vec<Monster> {
        self.iter()
            .filter(|m| m.drops().iter().any(|d| d.code == item_code))
            .collect_vec()
    }

    #[must_use]
    pub fn lowest_providing_xp_at(&self, level: u32) -> Option<Monster> {
        self.iter()
            .filter(|m| m.provides_xp_at(level))
            .min_by_key(Level::level)
    }

    #[must_use]
    pub fn highest_providing_exp(&self, level: u32) -> Option<Monster> {
        self.iter()
            .filter(|m| m.provides_xp_at(level))
            .max_by_key(Level::level)
    }

    #[must_use]
    pub fn is_event(&self, code: &str) -> bool {
        self.events
            .any(|e| e.content_code().is_some_and(|cc| cc == code))
    }
}

impl Cached<HashMap<String, Monster>> for MonstersClient {
    const FILE: &'static str = "monsters";

    fn directory(&self) -> &str {
        &self.directory
    }

    fn fetch_from_source(&self) -> HashMap<String, Monster> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.0.data.store(Arc::new(self.fetch_from_source()));
    }
}
