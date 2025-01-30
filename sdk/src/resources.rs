use std::{collections::HashMap, sync::LazyLock};
use crate::{events::EVENTS, PersistedData, API};
use artifactsmmo_openapi::models::ResourceSchema;
use itertools::Itertools;

pub static RESOURCES: LazyLock<Resources> = LazyLock::new(Resources::new);

pub struct Resources(HashMap<String, ResourceSchema>);

impl PersistedData<HashMap<String, ResourceSchema>> for Resources {
    fn data_from_api() -> HashMap<String, ResourceSchema> {
        API.resources
            .all(None, None, None, None)
            .unwrap()
            .into_iter()
            .map(|m| (m.code.clone(), m))
            .collect()
    }

    fn path() -> &'static str {
        ".cache/resources.json"
    }
}

impl Resources {
    fn new() -> Self {
        Self(Self::get_data())
    }

    pub fn get(&self, code: &str) -> Option<&ResourceSchema> {
        self.0.get(code)
    }

    pub fn all(&self) -> Vec<&ResourceSchema> {
        self.0.values().collect_vec()
    }

    pub fn dropping(&self, item: &str) -> Vec<&ResourceSchema> {
        self.0
            .values()
            .filter(|m| m.drops.iter().any(|d| d.code == item))
            .collect_vec()
    }

    pub fn is_event(&self, code: &str) -> bool {
        EVENTS.data.iter().any(|e| e.content.code == code)
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
