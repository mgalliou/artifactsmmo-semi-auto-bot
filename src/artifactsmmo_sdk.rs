use artifactsmmo_openapi::models::{CraftSchema, ItemEffectSchema, SimpleItemSchema};
use skill::Skill;

pub mod account;
pub mod api;
pub mod bank;
pub mod char_config;
pub mod character;
pub mod items;
pub mod maps;
pub mod monsters;
pub mod resources;
pub mod skill;

trait ItemSchemaExt {
    fn is_raw_mat(&self) -> bool;
    fn is_crafted_with(&self, code: &str) -> bool;
    fn mats(&self) -> Vec<SimpleItemSchema>;
    fn craft_schema(&self) -> Option<CraftSchema>;
    fn skill_to_craft(&self) -> Option<Skill>;
    fn effects(&self) -> Vec<&ItemEffectSchema>;
    fn damages(&self) -> i32;
}
