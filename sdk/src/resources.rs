use crate::{events::Events, PersistedData};
use artifactsmmo_api_wrapper::ArtifactApi;
use artifactsmmo_openapi::models::ResourceSchema;
use itertools::Itertools;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub struct Resources {
    data: RwLock<HashMap<String, Arc<ResourceSchema>>>,
    api: Arc<ArtifactApi>,
    events: Arc<Events>,
}

impl PersistedData<HashMap<String, Arc<ResourceSchema>>> for Resources {
    const PATH: &'static str = ".cache/resources.json";

    fn data_from_api(&self) -> HashMap<String, Arc<ResourceSchema>> {
        self.api
            .resources
            .all(None, None, None, None)
            .unwrap()
            .into_iter()
            .map(|r| (r.code.clone(), Arc::new(r)))
            .collect()
    }

    fn refresh_data(&self) {
        *self.data.write().unwrap() = self.data_from_api();
    }
}

impl Resources {
    pub(crate) fn new(api: Arc<ArtifactApi>, events: Arc<Events>) -> Self {
        let resources = Self {
            data: Default::default(),
            api,
            events,
        };
        *resources.data.write().unwrap() = resources.retrieve_data();
        resources
    }

    pub fn get(&self, code: &str) -> Option<Arc<ResourceSchema>> {
        self.data.read().unwrap().get(code).cloned()
    }

    pub fn all(&self) -> Vec<Arc<ResourceSchema>> {
        self.data.read().unwrap().values().cloned().collect_vec()
    }

    pub fn dropping(&self, item: &str) -> Vec<Arc<ResourceSchema>> {
        self.all()
            .into_iter()
            .filter(|m| m.drops.iter().any(|d| d.code == item))
            .collect_vec()
    }

    pub fn is_event(&self, code: &str) -> bool {
        self.events.all().iter().any(|e| e.content.code == code)
    }
}

pub trait ResourceSchemaExt {
    fn drop_rate(&self, item: &str) -> Option<i32>;
    fn max_drop_quantity(&self) -> i32;
}

impl ResourceSchemaExt for ResourceSchema {
    fn drop_rate(&self, item: &str) -> Option<i32> {
        self.drops.iter().find(|i| i.code == item).map(|i| i.rate)
    }

    fn max_drop_quantity(&self) -> i32 {
        self.drops.iter().map(|i| i.max_quantity).sum()
    }
}
