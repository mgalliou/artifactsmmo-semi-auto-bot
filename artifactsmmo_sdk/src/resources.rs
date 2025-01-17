use crate::{api::ResourcesApi, events::EVENTS, game_config::GAME_CONFIG, persist_data};
use artifactsmmo_openapi::models::ResourceSchema;
use lazy_static::lazy_static;
use log::error;
use std::{fs::read_to_string, path::Path, sync::Arc};

lazy_static! {
    pub static ref RESOURCES: Arc<Resources> = Arc::new(Resources::new());
}

#[derive(Default)]
pub struct Resources {
    pub data: Vec<ResourceSchema>,
}

impl Resources {
    fn new() -> Self {
        let api = ResourcesApi::new(&GAME_CONFIG.base_url);
        let path = Path::new(".cache/resources.json");
        let data = if path.exists() {
            let content = read_to_string(path).unwrap();
            serde_json::from_str(&content).unwrap()
        } else {
            let data = api
                .all(None, None, None, None)
                .expect("items to be retrieved from API.");
            if let Err(e) = persist_data(&data, path) {
                error!("failed to persist resources data: {}", e);
            }
            data
        };
        Resources { data }
    }

    pub fn get(&self, code: &str) -> Option<&ResourceSchema> {
        self.data.iter().find(|m| m.code == code)
    }

    pub fn dropping(&self, item: &str) -> Vec<&ResourceSchema> {
        self.data
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
