use crate::{
    CollectionClient, DataEntity, DropsItems, Persist, client::events::EventsClient,
    entities::Resource,
};
use api::ArtifactApi;
use itertools::Itertools;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, CollectionClient)]
pub struct ResourcesClient {
    data: RwLock<HashMap<String, Resource>>,
    api: Arc<ArtifactApi>,
    events: Arc<EventsClient>,
}

impl ResourcesClient {
    pub(crate) fn new(api: Arc<ArtifactApi>, events: Arc<EventsClient>) -> Self {
        let resources = Self {
            data: Default::default(),
            api,
            events,
        };
        *resources.data.write().unwrap() = resources.load();
        resources
    }

    pub fn dropping(&self, item_code: &str) -> Vec<Resource> {
        self.all()
            .into_iter()
            .filter(|r| r.drops().iter().any(|d| d.code == item_code))
            .collect_vec()
    }

    pub fn is_event(&self, code: &str) -> bool {
        self.events.all().iter().any(|e| e.content().code == code)
    }
}

impl Persist<HashMap<String, Resource>> for ResourcesClient {
    const PATH: &'static str = ".cache/resources.json";

    fn load_from_api(&self) -> HashMap<String, Resource> {
        self.api
            .resources
            .get_all()
            .unwrap()
            .into_iter()
            .map(|r| (r.code.clone(), Resource::new(r)))
            .collect()
    }

    fn refresh(&self) {
        *self.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for ResourcesClient {
    type Entity = Resource;
}
