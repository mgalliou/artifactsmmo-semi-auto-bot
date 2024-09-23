use artifactsmmo_openapi::models::{
    CraftSchema, ItemEffectSchema, MapContentSchema, MonsterSchema, ResourceSchema,
    SimpleItemSchema,
};
use items::{DamageType, Type};
use skill::Skill;

pub mod game;
pub mod account;
pub mod api;
pub mod bank;
pub mod char_config;
pub mod character;
pub mod config;
pub mod equipment;
pub mod items;
pub mod maps;
pub mod monsters;
pub mod resources;
pub mod skill;

trait ItemSchemaExt {
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

trait ResponseSchema {
    fn pretty(&self) -> String;
}

pub fn compute_damage(attack_damage: i32, damage_increase: i32, target_resistance: i32) -> f32 {
    attack_damage as f32
        * (1.0 + damage_increase as f32 / 100.0)
        * (1.0 - target_resistance as f32 / 100.0)
}
