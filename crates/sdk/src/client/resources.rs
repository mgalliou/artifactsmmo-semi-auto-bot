use crate::{
    Cached, CollectionClient, DropsItems,
    client::events::EventsClient,
    entities::{EventSchemaExt, Resource},
};
use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Default, Deref, CollectionClient)]
#[deref(forward)]
#[element(Resource)]
pub struct ResourcesClient(Arc<ResourcesClientInner>);

pub struct ResourcesClientInner {
    path: Box<str>,
    data: ArcSwap<HashMap<String, Resource>>,
    fetch: Box<dyn Fn() -> HashMap<String, Resource> + Send + Sync>,
    events: EventsClient,
}

impl Default for ResourcesClientInner {
    fn default() -> Self {
        Self {
            path: Box::from(".cache/resources.ron"),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("ResourcesClient not initialized")),
            events: EventsClient::default(),
        }
    }
}

impl ResourcesClient {
    pub(crate) fn new(
        path: &str,
        fetch: Box<dyn Fn() -> HashMap<String, Resource> + Send + Sync>,
        events: EventsClient,
    ) -> Self {
        Self(
            ResourcesClientInner {
                path: path.into(),
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
            Box::new(|| unreachable!("ResourcesClient::from_cache has no API fallback")),
            EventsClient::default(),
        );
        client.init();
        client
    }

    pub fn init(&self) {
        self.0.data.store(Arc::new(self.fetch()));
        info!("Resource client initilized");
    }

    #[must_use]
    pub fn dropping(&self, item_code: &str) -> Vec<Resource> {
        self.iter()
            .filter(|r| r.drops().iter().any(|d| d.code == item_code))
            .collect_vec()
    }

    #[must_use]
    pub fn is_event(&self, resource_code: &str) -> bool {
        self.events
            .any(|e| e.content_code().is_some_and(|cc| cc == resource_code))
    }
}

impl Cached<HashMap<String, Resource>> for ResourcesClient {
    fn path(&self) -> &str {
        &self.path
    }

    fn fetch_from_source(&self) -> HashMap<String, Resource> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.0.data.store(Arc::new(self.fetch_from_source()));
    }
}
