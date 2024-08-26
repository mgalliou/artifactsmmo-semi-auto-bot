use artifactsmmo_openapi::models::{
    craft_schema::Skill, CraftSchema, ItemEffectSchema, ItemSchema, SimpleItemSchema,
};
use enum_stringify::EnumStringify;
use strum_macros::EnumIter;

use super::{account::Account, api::items::ItemsApi};

pub struct Items {
    pub api: ItemsApi,
}

impl Items {
    pub fn new(account: &Account) -> Items {
        Items {
            api: ItemsApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
        }
    }

    // pub fn best_equipable_at_level(&self, level: i32, r#type: Type) -> Option<Vec<ItemSchema>> {
    //     let mut highest_lvl = 0;
    //     let mut best_schemas: Vec<ItemSchema> = vec![];

    //     todo!();
    //     best_schemas;
    // }

    pub fn best_craftable_at_level(&self, level: i32, skill: &str) -> Option<Vec<ItemSchema>> {
        let mut highest_lvl = 0;
        let mut best_schemas: Vec<ItemSchema> = vec![];

        match self
            .api
            .all(None, Some(level), None, None, Some(skill), None, None, None)
        {
            Ok(schemas) => {
                for schema in schemas.data {
                    if highest_lvl == 0 || highest_lvl <= schema.level {
                        highest_lvl = schema.level;
                        best_schemas.push(schema);
                    }
                }
                Some(best_schemas)
            }
            _ => None,
        }
    }

    pub fn craft_schema(&self, code: &str) -> Option<CraftSchema> {
        if let Ok(info) = self.api.info(code) {
            if let Some(Some(craft)) = info.data.item.craft {
                return Some(*craft);
            }
        };
        None
    }

    pub fn mats_for(&self, code: &str) -> Option<Vec<SimpleItemSchema>> {
        match self.craft_schema(code) {
            Some(schema) => schema.items,
            None => None,
        }
    }

    pub fn skill_to_craft(&self, code: &str) -> Option<Skill> {
        match self.craft_schema(code) {
            Some(schema) => schema.skill,
            None => None,
        }
    }

    pub fn effects_of(&self, code: &str) -> Option<Vec<ItemEffectSchema>> {
        match self.api.info(code) {
            Ok(info) => info.data.item.effects,
            Err(_) => None,
        }
    }

    pub fn damages(&self, code: &str) -> i32 {
        let mut total = 0;
        if let Some(effects) = self.effects_of(code) {
            for effect in effects {
                if effect.name.starts_with("attack_") {
                    total += effect.value;
                }
            }
        }
        total
    }
}

#[derive(Debug, PartialEq, EnumStringify, EnumIter)]
#[enum_stringify(case = "lower")]
pub enum Type {
    Consumable,
    BodyArmor,
    Weapon,
    Resource,
    LegArmor,
    Helmet,
    Boots,
    Shield,
    Amulet,
    Ring,
    Artifact,
}
