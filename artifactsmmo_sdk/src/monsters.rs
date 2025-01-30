use crate::{events::EVENTS, items::DamageType, PersistedData, API};
use artifactsmmo_openapi::models::MonsterSchema;
use itertools::Itertools;
use std::{collections::HashMap, sync::LazyLock};

pub static MONSTERS: LazyLock<Monsters> = LazyLock::new(Monsters::new);

pub struct Monsters(HashMap<String, MonsterSchema>);

impl PersistedData<HashMap<String, MonsterSchema>> for Monsters {
    fn data_from_api() -> HashMap<String, MonsterSchema> {
        API.monsters
            .all(None, None, None)
            .unwrap()
            .into_iter()
            .map(|m| (m.code.clone(), m))
            .collect()
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
        self.0.get(code)
    }

    pub fn all(&self) -> Vec<&MonsterSchema> {
        self.0.values().collect_vec()
    }

    pub fn dropping(&self, item: &str) -> Vec<&MonsterSchema> {
        self.0
            .values()
            .filter(|m| m.drops.iter().any(|d| d.code == item))
            .collect_vec()
    }

    pub fn lowest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.0
            .values()
            .filter(|m| m.level >= min && m.level <= level)
            .min_by_key(|m| m.level)
    }

    pub fn highest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        self.0
            .values()
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
