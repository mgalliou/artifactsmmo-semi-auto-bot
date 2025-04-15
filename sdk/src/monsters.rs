use crate::{events::Events, items::DamageType, PersistedData};
use artifactsmmo_api_wrapper::ArtifactApi;
use artifactsmmo_openapi::models::MonsterSchema;
use itertools::Itertools;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub struct Monsters {
    data: RwLock<HashMap<String, Arc<MonsterSchema>>>,
    api: Arc<ArtifactApi>,
    events: Arc<Events>,
}

impl PersistedData<HashMap<String, Arc<MonsterSchema>>> for Monsters {
    const PATH: &'static str = ".cache/monsters.json";

    fn data_from_api(&self) -> HashMap<String, Arc<MonsterSchema>> {
        self.api
            .monsters
            .all(None, None, None)
            .unwrap()
            .into_iter()
            .map(|m| (m.code.clone(), Arc::new(m)))
            .collect()
    }

    fn refresh_data(&self) {
        *self.data.write().unwrap() = self.data_from_api();
    }
}

impl Monsters {
    pub(crate) fn new(api: Arc<ArtifactApi>, events: Arc<Events>) -> Self {
        let monsters = Self {
            data: Default::default(),
            api,
            events,
        };
        *monsters.data.write().unwrap() = monsters.retrieve_data();
        monsters
    }

    pub fn get(&self, code: &str) -> Option<Arc<MonsterSchema>> {
        self.data.read().unwrap().get(code).cloned()
    }

    pub fn all(&self) -> Vec<Arc<MonsterSchema>> {
        self.data.read().unwrap().values().cloned().collect_vec()
    }

    pub fn dropping(&self, item: &str) -> Vec<Arc<MonsterSchema>> {
        self.all()
            .into_iter()
            .filter(|m| m.drops.iter().any(|d| d.code == item))
            .collect_vec()
    }

    pub fn lowest_providing_exp(&self, level: i32) -> Option<Arc<MonsterSchema>> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.all()
            .into_iter()
            .filter(|m| m.level >= min && m.level <= level)
            .min_by_key(|m| m.level)
    }

    pub fn highest_providing_exp(&self, level: i32) -> Option<Arc<MonsterSchema>> {
        self.all()
            .into_iter()
            .filter(|m| m.level <= level)
            .max_by_key(|m| m.level)
    }

    pub fn is_event(&self, code: &str) -> bool {
        self.events.all().iter().any(|e| e.content.code == code)
    }
}

pub trait MonsterSchemaExt {
    fn resistance(&self, r#type: DamageType) -> i32;
    fn attack_damage(&self, r#type: DamageType) -> i32;
    fn drop_rate(&self, item: &str) -> Option<i32>;
    fn max_drop_quantity(&self) -> i32;
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

    fn max_drop_quantity(&self) -> i32 {
        self.drops.iter().map(|i| i.max_quantity).sum()
    }
}

#[cfg(test)]
mod tests {
    use crate::MONSTERS;

    use super::*;

    #[test]
    fn max_drop_quantity() {
        assert_eq!(MONSTERS.get("cow").unwrap().max_drop_quantity(), 4);
    }
}
