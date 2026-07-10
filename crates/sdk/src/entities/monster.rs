use crate::{
    CanProvideXp, Code, DropRateSchemaExt, HasDropTable, Level,
    simulator::{DamageType, HasEffects},
};
use openapi::models::{MonsterSchema, MonsterType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Monster(Arc<MonsterSchema>);

impl Monster {
    #[must_use]
    pub(crate) fn new(schema: MonsterSchema) -> Self {
        Self(Arc::new(schema))
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.0.name
    }

    #[must_use]
    pub fn is_boss(&self) -> bool {
        self.0.r#type == MonsterType::Boss
    }
}

impl HasDropTable for Monster {
    fn drops(&self) -> &[impl DropRateSchemaExt] {
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

    fn effect_value(&self, effect: &str) -> i32 {
        self.0
            .effects
            .iter()
            .flatten()
            .find(|e| e.code == effect)
            .map_or(0, |e| e.value)
    }
}

impl CanProvideXp for Monster {}
