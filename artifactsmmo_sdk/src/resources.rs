use std::sync::LazyLock;

use crate::{events::EVENTS, PersistedData, API};
use artifactsmmo_openapi::models::ResourceSchema;

pub static RESOURCES: LazyLock<Resources> = LazyLock::new(Resources::new);

pub struct Resources(Vec<ResourceSchema>);

impl PersistedData<Vec<ResourceSchema>> for Resources {
    fn data_from_api() -> Vec<ResourceSchema> {
        API.resources.all(None, None, None, None).unwrap()
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
        self.0.iter().find(|m| m.code == code)
    }

    pub fn all(&self) -> &Vec<ResourceSchema> {
        &self.0
    }

    pub fn dropping(&self, item: &str) -> Vec<&ResourceSchema> {
        self.0
            .iter()
            .filter(|r| r.drops.iter().any(|d| d.code == item))
            .collect::<Vec<_>>()
    }

    pub fn is_event(&self, code: &str) -> bool {
        EVENTS.data.iter().any(|e| e.content.code == code)
    }
}

pub trait ResourceSchemaExt {
    fn drop_rate(&self, item: &str) -> Option<i32>;
}

impl ResourceSchemaExt for ResourceSchema {
    fn drop_rate(&self, item: &str) -> Option<i32> {
        self.drops.iter().find(|i| i.code == item).map(|i| i.rate)
    }
}
