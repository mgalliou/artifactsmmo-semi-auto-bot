use crate::{events::EVENTS, PersistedData, API};
use artifactsmmo_openapi::models::ResourceSchema;
use itertools::Itertools;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, RwLock},
};

pub static RESOURCES: LazyLock<Resources> = LazyLock::new(Resources::new);

pub struct Resources(RwLock<HashMap<String, Arc<ResourceSchema>>>);

impl PersistedData<HashMap<String, Arc<ResourceSchema>>> for Resources {
    const PATH: &'static str = ".cache/resources.json";

    fn data_from_api() -> HashMap<String, Arc<ResourceSchema>> {
        API.resources
            .all(None, None, None, None)
            .unwrap()
            .into_iter()
            .map(|r| (r.code.clone(), Arc::new(r)))
            .collect()
    }

    fn refresh_data(&self) {
        *self.0.write().unwrap() = Self::data_from_api();
    }
}

impl Resources {
    fn new() -> Self {
        Self(RwLock::new(Self::retrieve_data()))
    }

    pub fn get(&self, code: &str) -> Option<Arc<ResourceSchema>> {
        self.0.read().unwrap().get(code).cloned()
    }

    pub fn all(&self) -> Vec<Arc<ResourceSchema>> {
        self.0.read().unwrap().values().cloned().collect_vec()
    }

    pub fn dropping(&self, item: &str) -> Vec<Arc<ResourceSchema>> {
        self.all()
            .into_iter()
            .filter(|m| m.drops.iter().any(|d| d.code == item))
            .collect_vec()
    }

    pub fn is_event(&self, code: &str) -> bool {
        EVENTS.all().iter().any(|e| e.content.code == code)
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
