use crate::{
    CanProvideXp, Code, DropsItems, Level,
    simulator::{DamageType, HasEffects},
};
use itertools::Itertools;
use openapi::models::{DropRateSchema, MonsterSchema, MonsterType, SimpleEffectSchema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Monster(Arc<MonsterSchema>);

impl Monster {
    pub fn new(schema: MonsterSchema) -> Self {
        Self(Arc::new(schema))
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn is_boss(&self) -> bool {
        self.0.r#type == MonsterType::Boss
    }
}

impl DropsItems for Monster {
    fn drops(&self) -> &Vec<DropRateSchema> {
        &self.0.drops
    }
}

impl Level for Monster {
    fn level(&self) -> u32 {
        self.0.level as u32
    }
}

impl Code for Monster {
    fn code(&self) -> &str {
        &self.0.code
    }
}

impl HasEffects for Monster {
    fn health(&self) -> i32 {
        self.0.hp
    }

    fn attack_dmg(&self, r#type: DamageType) -> i32 {
        match r#type {
            DamageType::Fire => self.0.attack_fire,
            DamageType::Earth => self.0.attack_earth,
            DamageType::Water => self.0.attack_water,
            DamageType::Air => self.0.attack_air,
        }
    }

    fn critical_strike(&self) -> i32 {
        self.0.critical_strike
    }

    fn res(&self, r#type: DamageType) -> i32 {
        match r#type {
            DamageType::Fire => self.0.res_fire,
            DamageType::Earth => self.0.res_earth,
            DamageType::Water => self.0.res_water,
            DamageType::Air => self.0.res_air,
        }
    }

    fn initiative(&self) -> i32 {
        self.0.initiative
    }

    fn effects(&self) -> Vec<SimpleEffectSchema> {
        self.0.effects.iter().flatten().cloned().collect_vec()
    }
}

impl CanProvideXp for Monster {}
