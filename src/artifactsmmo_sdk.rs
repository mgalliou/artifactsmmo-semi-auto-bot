use artifactsmmo_openapi::models::{
    CraftSchema, ItemEffectSchema, MapContentSchema, MonsterSchema, ResourceSchema, SimpleItemSchema
};
use items::{DamageType, Type};
use skill::Skill;

pub mod account;
pub mod api;
pub mod bank;
pub mod char_config;
pub mod character;
pub mod config;
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
    fn attack_damage_against(&self, monster: &MonsterSchema) -> i32;
    fn total_damage_increase(&self) -> i32;
    fn damage_increase(&self, r#type: DamageType) -> i32;
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
}

trait ResponseSchema {
    fn pretty(&self) -> String;
}

