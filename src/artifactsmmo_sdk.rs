use artifactsmmo_openapi::models::{
    CharacterSchema, CraftSchema, ItemEffectSchema, ItemSchema, MapContentSchema, MonsterSchema,
    SimpleItemSchema,
};
use downcast_rs::{impl_downcast, Downcast};
use fs_extra::file::{read_to_string, write_all};
use items::{DamageType, Type};
use serde::{Deserialize, Serialize};
use skill::Skill;
use std::{fmt::Display, path::Path};

pub mod account;
pub mod api;
pub mod bank;
pub mod char_config;
pub mod character;
pub mod events;
pub mod fight_simulator;
pub mod game;
pub mod game_config;
pub mod gear;
pub mod gear_finder;
pub mod inventory;
pub mod items;
pub mod leveling_helper;
pub mod maps;
pub mod monsters;
pub mod orderboard;
pub mod resources;
pub mod skill;
pub mod tasks;

trait ItemSchemaExt {
    fn name(&self) -> String;
    fn r#type(&self) -> Type;
    fn is_of_type(&self, r#type: Type) -> bool;
    fn is_crafted_with(&self, item: &str) -> bool;
    fn is_crafted_from_task(&self) -> bool;
    fn mats(&self) -> Vec<SimpleItemSchema>;
    fn craft_schema(&self) -> Option<CraftSchema>;
    fn skill_to_craft(&self) -> Option<Skill>;
    fn effects(&self) -> Vec<&ItemEffectSchema>;
    fn attack_damage(&self, r#type: DamageType) -> i32;
    fn attack_damage_against(&self, monster: &MonsterSchema) -> f32;
    fn damage_increase(&self, r#type: DamageType) -> i32;
    fn resistance(&self, r#type: DamageType) -> i32;
    fn health(&self) -> i32;
    fn haste(&self) -> i32;
    fn skill_cooldown_reduction(&self, skijll: Skill) -> i32;
    fn heal(&self) -> i32;
    fn restore(&self) -> i32;
    fn inventory_space(&self) -> i32;
    fn is_consumable(&self, level: i32) -> bool;
    fn damage_increase_against_with(&self, monster: &MonsterSchema, weapon: &ItemSchema) -> f32;
    fn damage_reduction_against(&self, monster: &MonsterSchema) -> f32;
}

trait MapSchemaExt {
    fn content(&self) -> Option<MapContentSchema>;
    fn content_is(&self, code: &str) -> bool;
    fn pretty(&self) -> String;
}

trait ResourceSchemaExt {
    fn drop_rate(&self, item: &str) -> Option<i32>;
}

trait MonsterSchemaExt {
    fn resistance(&self, r#type: DamageType) -> i32;
    fn attack_damage(&self, r#type: DamageType) -> i32;
    fn drop_rate(&self, item: &str) -> Option<i32>;
}

trait ActiveEventSchemaExt {
    fn content_code(&self) -> &String;
}

pub trait ResponseSchema: Downcast {
    fn character(&self) -> &CharacterSchema;
    fn pretty(&self) -> String;
}
impl_downcast!(ResponseSchema);

trait FightSchemaExt {
    fn amount_of(&self, item: &str) -> i32;
}

trait SkillSchemaExt {
    fn amount_of(&self, item: &str) -> i32;
}

trait SkillInfoSchemaExt {
    fn amount_of(&self, item: &str) -> i32;
}

trait RewardsSchemaExt {
    fn amount_of(&self, item: &str) -> i32;
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorResponseSchema {
    error: ApiErrorSchema,
}

impl Display for ApiErrorResponseSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error.message, self.error.code)
    }
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorSchema {
    code: i32,
    message: String,
}

pub trait ApiRequestError {}

pub fn retreive_data<T: for<'a> Deserialize<'a>>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&read_to_string(path)?)?)
}

pub fn persist_data<T: Serialize>(data: T, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(write_all(path, &serde_json::to_string_pretty(&data)?)?)
}
