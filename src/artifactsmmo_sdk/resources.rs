use super::{api::resources::ResourcesApi, config::Config, skill::Skill};
use artifactsmmo_openapi::models::ResourceSchema;

pub struct Resources {
    pub data: Vec<ResourceSchema>,
}

impl Resources {
    pub fn new(config: &Config) -> Resources {
        let api = ResourcesApi::new(&config.base_url, &config.token);
        Resources {
            data: api.all(None, None, None, None).unwrap().clone(),
        }
    }

    pub fn get(&self, code: &str) -> Option<&ResourceSchema> {
        self.data.iter().find(|m| m.code == code)
    }

    pub fn dropping(&self, code: &str) -> Vec<&ResourceSchema> {
        self.data
            .iter()
            .filter(|r| r.drops.iter().any(|d| d.code == code))
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
}
