use artifactsmmo_openapi::models::{
    CharacterSchema, CraftSchema, ItemEffectSchema, ItemSchema, MapContentSchema, MonsterSchema,
    ResourceSchema, SimpleItemSchema,
};
use downcast_rs::{impl_downcast, Downcast};
use fs_extra::file::{read_to_string, write_all};
use items::{DamageType, Type};
use serde::{Deserialize, Serialize};
use skill::Skill;
use std::path::Path;

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
pub mod items;
pub mod maps;
pub mod monsters;
pub mod orderboard;
pub mod resources;
pub mod skill;
pub mod tasks;

trait ItemSchemaExt {
    fn name(&self) -> String;
    fn is_raw_mat(&self) -> bool;
    fn is_of_type(&self, r#type: Type) -> bool;
    fn is_crafted_with(&self, code: &str) -> bool;
    fn mats(&self) -> Vec<SimpleItemSchema>;
    fn craft_schema(&self) -> Option<CraftSchema>;
    fn skill_to_craft(&self) -> Option<Skill>;
    fn effects(&self) -> Vec<&ItemEffectSchema>;
    fn total_attack_damage(&self) -> i32;
    fn attack_damage(&self, r#type: DamageType) -> i32;
    fn attack_damage_against(&self, monster: &MonsterSchema) -> f32;
    fn total_damage_increase(&self) -> i32;
    fn damage_increase(&self, r#type: DamageType) -> i32;
    fn damage_from(&self, monster: &MonsterSchema) -> f32;
    fn resistance(&self, r#type: DamageType) -> i32;
    fn total_resistance(&self) -> i32;
    fn health(&self) -> i32;
    fn haste(&self) -> i32;
    fn skill_cooldown_reduction(&self, skijll: Skill) -> i32;
    fn heal(&self) -> i32;
    fn damage_increase_against_with(&self, monster: &MonsterSchema, weapon: &ItemSchema) -> f32;
    fn damage_reduction_against(&self, monster: &MonsterSchema) -> f32;
}

trait MapSchemaExt {
    fn has_one_of_resource(&self, resources: &[&ResourceSchema]) -> bool;
    fn content(&self) -> Option<MapContentSchema>;
    fn content_is(&self, code: &str) -> bool;
    fn pretty(&self) -> String;
}

trait MonsterSchemaExt {
    fn resistance(&self, r#type: DamageType) -> i32;
    fn attack_damage(&self, r#type: DamageType) -> i32;
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
    fn amount_of(&self, code: &str) -> i32;
}

trait SkillSchemaExt {
    fn amount_of(&self, code: &str) -> i32;
}

trait SkillInfoSchemaExt {
    fn amount_of(&self, code: &str) -> i32;
}

trait TaskRewardsSchemaExt {
    fn amount_of(&self, code: &str) -> i32;
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorResponseSchema {
    error: ApiErrorSchema,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorSchema {
    code: i32,
    message: String,
}

pub trait ApiRequestError {}

/// Compute the average damage an attack will do against the given `target_resistance`. Block
/// chance is considered as a global damage reduction (30 resistence reduce the computed damage by
/// 3%).
pub fn average_dmg(attack_damage: i32, damage_increase: i32, target_resistance: i32) -> f32 {
    let mut dmg = attack_damage as f32
        + (attack_damage as f32 * damage_increase as f32 * 0.01);
    dmg -= dmg * target_resistance as f32 * 0.01;
    // TODO: include this in a different function and rename this one
    //if target_resistance > 0 {
    //    dmg *= 1.0 - (target_resistance as f32 / 1000.0)
    //};
    dmg
}

pub fn retreive_data<T: for<'a> Deserialize<'a>>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&read_to_string(path)?)?)
}

pub fn persist_data<T: Serialize>(
    data: T,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(write_all(path, &serde_json::to_string_pretty(&data)?)?)
}
