use crate::{
    CollectionClient, Data, DataEntity, DropsItems, Persist, client::events::EventsClient,
    entities::Resource,
};
use api::ArtifactApi;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
pub struct ResourcesClient(Arc<ResourcesClientInner>);

#[derive(Default, Debug)]
pub struct ResourcesClientInner {
    api: ArtifactApi,
    data: RwLock<Arc<HashMap<String, Resource>>>,
    events: EventsClient,
}

impl ResourcesClient {
    pub(crate) fn new(api: ArtifactApi, events: EventsClient) -> Self {
        Self(
            ResourcesClientInner {
                api,
                data: RwLock::default(),
                events,
            }
            .into(),
        )
    }

    pub fn init(&self) {
        *self.data_mut() = Arc::new(self.load());
        info!("Resource client initilized");
    }

    #[must_use]
    pub fn dropping(&self, item_code: &str) -> Vec<Resource> {
        self.all()
            .into_iter()
            .filter(|r| r.drops().iter().any(|d| d.code == item_code))
            .collect_vec()
    }

    #[must_use]
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
        *self.data_mut() = Arc::new(self.load_from_api());
    }
}

impl DataEntity for ResourcesClient {
    type Entity = Resource;
}
