use artifactsmmo_openapi::models::{
    craft_schema::Skill, CraftSchema, GeItemSchema, ItemEffectSchema, ItemSchema, SimpleItemSchema,
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

    pub fn lower_providing_exp(
        &self,
        level: i32,
        skill: super::skill::Skill,
    ) -> Option<Vec<ItemSchema>> {
        let min = if level > 11 { level - 10 } else { 1 };
        let items = self
            .api
            .all(
                Some(min),
                Some(level),
                Some(&skill.to_string()),
                None,
                None,
                None,
            )
            .ok()?;
        let min_level = items.iter().min_by_key(|i| i.level).map(|i| i.level)?;
        Some(
            items
                .iter()
                .filter(|i| i.level == min_level)
                .cloned()
                .collect(),
        )
    }

    pub fn best_craftable_at_level(
        &self,
        level: i32,
        skill: super::skill::Skill,
    ) -> Option<Vec<ItemSchema>> {
        let items = self
            .api
            .all(
                None,
                Some(level),
                Some(&skill.to_string()),
                None,
                None,
                None,
            )
            .ok()?;
        let max_level = items.iter().max_by_key(|i| i.level).map(|i| i.level)?;
        Some(
            items
                .iter()
                .filter(|i| i.level == max_level)
                .cloned()
                .collect(),
        )
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

    pub fn ge_info(&self, code: &str) -> Option<Box<GeItemSchema>> {
        let schema = self.api.info(code).unwrap();
        match schema.data.ge {
            Some(Some(ge)) => Some(ge),
            Some(None) => None,
            None => None,
        }
    }

    pub fn ge_mats_price(&self, code: &str) -> i32 {
        let mut total = 0;
        for mat in self.mats_for(code).unwrap() {
            total += self
                .ge_info(&mat.code)
                .map_or(0, |i| i.buy_price.unwrap_or(0));
        }
        total
    }

    pub fn mats_quantity_for(&self, code: &str) -> i32 {
        self.mats_for(code)
            .map(|mats| mats.iter().map(|mat| mat.quantity).sum::<i32>())
            .unwrap_or(0)
    }

    pub fn skill_to_craft(&self, code: &str) -> Option<super::skill::Skill> {
        self.craft_schema(code)
            .and_then(|schema| schema.skill)
            .map(|skill| self.schema_skill_to_skill(skill))
    }

    fn schema_skill_to_skill(&self, skill: Skill) -> super::skill::Skill {
        match skill {
            Skill::Weaponcrafting => super::skill::Skill::Weaponcrafting,
            Skill::Gearcrafting => super::skill::Skill::Gearcrafting,
            Skill::Jewelrycrafting => super::skill::Skill::Jewelrycrafting,
            Skill::Cooking => super::skill::Skill::Cooking,
            Skill::Woodcutting => super::skill::Skill::Woodcutting,
            Skill::Mining => super::skill::Skill::Mining,
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
