use super::{account::Account, api::items::ItemsApi, monsters::Monsters, resources::Resources};
use artifactsmmo_openapi::models::{
    craft_schema::Skill, CraftSchema, GeItemSchema, ItemEffectSchema, ItemSchema, SimpleItemSchema,
};
use enum_stringify::EnumStringify;
use itertools::Itertools;
use std::{sync::Arc, vec::Vec};
use strum_macros::EnumIter;

pub struct Items {
    pub data: Vec<ItemSchema>,
    pub api: ItemsApi,
    pub resources: Arc<Resources>,
    pub monsters: Arc<Monsters>,
}

impl Items {
    pub fn new(account: &Account, resources: Arc<Resources>, monsters: Arc<Monsters>) -> Items {
        let api = ItemsApi::new(
            &account.configuration.base_path,
            &account.configuration.bearer_access_token.clone().unwrap(),
        );
        Items {
            data: api.all(None, None, None, None, None, None).unwrap().clone(),
            api,
            resources,
            monsters,
        }
    }

    // pub fn best_equipable_at_level(&self, level: i32, r#type: Type) -> Option<Vec<ItemSchema>> {
    //     let mut highest_lvl = 0;
    //     let mut best_schemas: Vec<ItemSchema> = vec![];

    //     todo!();
    //     best_schemas;
    // }

    pub fn best_for_leveling(&self, level: i32, skill: super::skill::Skill) -> Option<ItemSchema> {
        self.providing_exp(level, skill)
            .iter()
            .filter(|i| !self.is_crafted_with(&i.code, "jasper_crystal"))
            .min_set_by_key(|i| (self.base_mats_drop_rate(&i.code) * 100.0) as i32)
            .into_iter()
            .min_set_by_key(|i| self.base_mats_buy_price(&i.code))
            .into_iter()
            .max_by_key(|i| i.level)
            .cloned()
    }

    pub fn providing_exp(&self, level: i32, skill: super::skill::Skill) -> Vec<ItemSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.data
            .iter()
            .filter(|i| i.level >= min && i.level <= level)
            .filter(|i| {
                self.craft_schema(&i.code).is_some_and(|c| {
                    c.skill
                        .is_some_and(|s| Items::schema_skill_to_skill(s) == skill)
                })
            })
            .cloned()
            .collect_vec()
    }

    pub fn lowest_providing_exp(&self, level: i32, skill: super::skill::Skill) -> Vec<ItemSchema> {
        self.providing_exp(level, skill)
            .iter()
            .min_set_by_key(|i| i.level)
            .into_iter()
            .cloned()
            .collect_vec()
    }

    pub fn highest_providing_exp(&self, level: i32, skill: super::skill::Skill) -> Vec<ItemSchema> {
        self.providing_exp(level, skill)
            .iter()
            .max_set_by_key(|i| i.level)
            .into_iter()
            .cloned()
            .collect_vec()
    }

    pub fn craft_schema(&self, code: &str) -> Option<CraftSchema> {
        self.get(code)?.craft.clone()?.map(|c| (*c))
    }

    pub fn is_craftable(&self, code: &str) -> bool {
        self.craft_schema(code).is_some()
    }

    pub fn base_mats_for(&self, code: &str) -> Option<Vec<SimpleItemSchema>> {
        let mut base_mats: Vec<SimpleItemSchema> = vec![];
        for mat in self.mats_for(code)? {
            match self.base_mats_for(&mat.code) {
                Some(mut b) => {
                    b.iter_mut().for_each(|b| b.quantity *= mat.quantity);
                    base_mats.append(&mut b)
                }
                None => base_mats.push(mat),
            }
        }
        Some(base_mats)
    }

    pub fn mats_for(&self, code: &str) -> Option<Vec<SimpleItemSchema>> {
        self.craft_schema(code)?.items
    }

    pub fn is_crafted_with(&self, code: &str, mat: &str) -> bool {
        self.base_mats_for(code)
            .is_some_and(|mats| mats.iter().any(|m| m.code == mat))
    }

    pub fn with_material(&self, code: &str) -> Vec<ItemSchema> {
        self.data
            .iter()
            .filter(|i| {
                self.craft_schema(&i.code).is_some_and(|c| {
                    c.items
                        .is_some_and(|items| items.iter().any(|i| i.code == code))
                })
            })
            .cloned()
            .collect_vec()
    }

    pub fn ge_info(&self, code: &str) -> Option<Box<GeItemSchema>> {
        self.api.info(code).ok()?.data.ge?
    }

    pub fn base_mats_buy_price(&self, code: &str) -> i32 {
        let i = self.base_mats_for(code).map_or(0, |mats| {
            mats.iter()
                .map(|mat| {
                    self.ge_info(&mat.code)
                        .map_or(0, |i| i.buy_price.unwrap_or(0) * mat.quantity)
                })
                .sum()
        });
        println!("total price for {}: {}", code, i);
        i
    }

    pub fn mats_quantity_for(&self, code: &str) -> i32 {
        self.mats_for(code)
            .map(|mats| mats.iter().map(|mat| mat.quantity).sum())
            .unwrap_or(0)
    }

    pub fn drop_rate(&self, code: &str) -> i32 {
        let mut rate: i32 = 0;
        if let Some(info) = self.get(code) {
            if info.subtype == "mob" {
                if let Some(monsters) = self.monsters.dropping(code) {
                    rate = monsters
                        .iter()
                        .map(|m| {
                            m.drops
                                .iter()
                                .find(|d| d.code == code)
                                .map(|d| d.rate)
                                .unwrap_or(0)
                        })
                        .min()
                        .unwrap_or(0)
                }
            } else if let Some(resources) = self.resources.dropping(code) {
                rate = resources
                    .iter()
                    .map(|m| {
                        m.drops
                            .iter()
                            .find(|d| d.code == code)
                            .map(|d| d.rate)
                            .unwrap_or(0)
                    })
                    .min()
                    .unwrap_or(0)
            }
        }
        println!("drop rate for {}: {}", code, rate);
        rate
    }

    pub fn base_mats_drop_rate(&self, code: &str) -> f32 {
        if let Some(mats) = self.base_mats_for(code) {
            let total_mats: i32 = mats.iter().map(|m| m.quantity).sum();
            println!("total mats for {}: {}", code, total_mats);
            let sum: i32 = mats
                .iter()
                .map(|m| self.drop_rate(&m.code) * m.quantity)
                .sum();
            println!("sum for {}: {}", code, sum);
            let average: f32 = sum as f32 / total_mats as f32;
            println!("average drop rate for {}: {}", code, average);
            return average;
        }
        0.0
    }

    pub fn skill_to_craft(&self, code: &str) -> Option<super::skill::Skill> {
        self.craft_schema(code)
            .and_then(|schema| schema.skill)
            .map(Items::schema_skill_to_skill)
    }

    pub fn effects_of(&self, code: &str) -> Option<Vec<ItemEffectSchema>> {
        self.get(code)?.effects.clone()
    }

    pub fn damages(&self, code: &str) -> i32 {
        self.effects_of(code).map_or(0, |e| {
            e.iter()
                .filter(|e| !e.name.starts_with("attack_"))
                .map(|e| e.value)
                .sum()
        })
    }

    pub fn schema_skill_to_skill(skill: Skill) -> super::skill::Skill {
        match skill {
            Skill::Weaponcrafting => super::skill::Skill::Weaponcrafting,
            Skill::Gearcrafting => super::skill::Skill::Gearcrafting,
            Skill::Jewelrycrafting => super::skill::Skill::Jewelrycrafting,
            Skill::Cooking => super::skill::Skill::Cooking,
            Skill::Woodcutting => super::skill::Skill::Woodcutting,
            Skill::Mining => super::skill::Skill::Mining,
        }
    }

    pub fn get(&self, code: &str) -> Option<&ItemSchema> {
        self.data.iter().find(|m| m.code == code)
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
