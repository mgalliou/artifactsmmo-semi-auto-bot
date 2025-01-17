use crate::{
    api::monsters::MonstersApi, events::EVENTS, game_config::GAME_CONFIG, items::DamageType,
    persist_data, retreive_data,
};
use artifactsmmo_openapi::models::MonsterSchema;
use lazy_static::lazy_static;
use log::error;
use std::{path::Path, sync::Arc};

lazy_static! {
    pub static ref MONSTERS: Arc<Monsters> = Arc::new(Monsters::new());
}

#[derive(Default)]
pub struct Monsters {
    pub data: Vec<MonsterSchema>,
}

impl Monsters {
    fn new() -> Monsters {
        let api = MonstersApi::new(GAME_CONFIG.token.as_str());
        let path = Path::new(".cache/monsters.json");
        let data = if let Ok(data) = retreive_data::<Vec<MonsterSchema>>(path) {
            data
        } else {
            let data = api
                .all(None, None, None)
                .expect("items to be retrieved from API.");
            if let Err(e) = persist_data(&data, path) {
                error!("failed to persist monsters data: {}", e);
            }
            data
        };
        Monsters { data }
    }
    pub fn get(&self, code: &str) -> Option<&MonsterSchema> {
        self.data.iter().find(|m| m.code == code)
    }

    pub fn dropping(&self, item: &str) -> Vec<&MonsterSchema> {
        self.data
            .iter()
            .filter(|m| m.drops.iter().any(|d| d.code == item))
            .collect::<Vec<_>>()
    }

    pub fn lowest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.data
            .iter()
            .filter(|m| m.level >= min && m.level <= level)
            .min_by_key(|m| m.level)
    }

    pub fn highest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        self.data
            .iter()
            .filter(|m| m.level <= level)
            .max_by_key(|m| m.level)
    }

    pub fn is_event(&self, code: &str) -> bool {
        EVENTS.data.iter().any(|e| e.content.code == code)
    }
}

pub trait MonsterSchemaExt {
    fn resistance(&self, r#type: DamageType) -> i32;
    fn attack_damage(&self, r#type: DamageType) -> i32;
    fn drop_rate(&self, item: &str) -> Option<i32>;
}

impl MonsterSchemaExt for MonsterSchema {
    fn attack_damage(&self, r#type: DamageType) -> i32 {
        match r#type {
            DamageType::Air => self.attack_air,
            DamageType::Earth => self.attack_earth,
            DamageType::Fire => self.attack_fire,
            DamageType::Water => self.attack_water,
        }
    }

    fn resistance(&self, r#type: DamageType) -> i32 {
        match r#type {
            DamageType::Air => self.res_air,
            DamageType::Earth => self.res_earth,
            DamageType::Fire => self.res_fire,
            DamageType::Water => self.res_water,
        }
    }

    fn drop_rate(&self, item: &str) -> Option<i32> {
        self.drops.iter().find(|i| i.code == item).map(|i| i.rate)
    }
}
