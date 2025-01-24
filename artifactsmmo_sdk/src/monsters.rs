use crate::{events::EVENTS, items::DamageType, PersistedData, API};
use artifactsmmo_openapi::models::MonsterSchema;
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref MONSTERS: Arc<Monsters> = Arc::new(Monsters::new());
}

pub struct Monsters(Vec<MonsterSchema>);

impl PersistedData<Vec<MonsterSchema>> for Monsters {
    fn data_from_api() -> Vec<MonsterSchema> {
        API.monsters.all(None, None, None).unwrap()
    }

    fn path() -> &'static str {
        ".cache/monsters.json"
    }
}

impl Monsters {
    fn new() -> Self {
        Self(Self::get_data())
    }

    pub fn get(&self, code: &str) -> Option<&MonsterSchema> {
        self.0.iter().find(|m| m.code == code)
    }

    pub fn all(&self) -> &Vec<MonsterSchema> {
        &self.0
    }

    pub fn dropping(&self, item: &str) -> Vec<&MonsterSchema> {
        self.0
            .iter()
            .filter(|m| m.drops.iter().any(|d| d.code == item))
            .collect::<Vec<_>>()
    }

    pub fn lowest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.0
            .iter()
            .filter(|m| m.level >= min && m.level <= level)
            .min_by_key(|m| m.level)
    }

    pub fn highest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        self.0
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
