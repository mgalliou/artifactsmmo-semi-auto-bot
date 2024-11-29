use super::{
    api::resources::ResourcesApi, events::Events, game_config::GameConfig, persist_data,
    skill::Skill, ResourceSchemaExt,
};
use artifactsmmo_openapi::models::ResourceSchema;
use log::error;
use std::{fs::read_to_string, path::Path, sync::Arc};

#[derive(Default)]
pub struct Resources {
    data: Vec<ResourceSchema>,
    events: Arc<Events>,
}

impl Resources {
    pub fn new(config: &GameConfig, events: &Arc<Events>) -> Resources {
        let api = ResourcesApi::new(&config.base_url);
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
        Resources {
            data,
            events: events.clone(),
        }
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

    pub fn lowest_providing_exp(&self, level: i32, skill: Skill) -> Option<&ResourceSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.data
            .iter()
            .filter(|r| Skill::from(r.skill) == skill)
            .filter(|r| r.level >= min && r.level <= level)
            .min_by_key(|r| r.level)
    }

    pub fn highest_providing_exp(&self, level: i32, skill: Skill) -> Option<&ResourceSchema> {
        self.data
            .iter()
            .filter(|r| Skill::from(r.skill) == skill)
            .filter(|r| r.level <= level)
            .max_by_key(|r| r.level)
    }

    pub fn is_event(&self, code: &str) -> bool {
        self.events.data.iter().any(|e| e.content.code == code)
    }
}

impl ResourceSchemaExt for ResourceSchema {
    fn drop_rate(&self, item: &str) -> Option<i32> {
        self.drops.iter().find(|i| i.code == item).map(|i| i.rate)
    }
}
