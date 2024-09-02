use artifactsmmo_openapi::models::{
    craft_schema::Skill, CraftSchema, DropRateSchema, GeItemSchema, ItemEffectSchema, ItemSchema,
    SimpleItemSchema,
};
use enum_stringify::EnumStringify;
use itertools::Itertools;
use strum_macros::EnumIter;

use super::{account::Account, api::items::ItemsApi, monsters::Monsters, resources::Resources};

pub struct Items {
    pub api: ItemsApi,
    pub monsters: Monsters,
    pub resources: Resources,
}

impl Items {
    pub fn new(account: &Account) -> Items {
        Items {
            api: ItemsApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            monsters: Monsters::new(account),
            resources: Resources::new(account),
        }
    }

    // pub fn best_equipable_at_level(&self, level: i32, r#type: Type) -> Option<Vec<ItemSchema>> {
    //     let mut highest_lvl = 0;
    //     let mut best_schemas: Vec<ItemSchema> = vec![];

    //     todo!();
    //     best_schemas;
    // }

    pub fn best_for_leveling(&self, level: i32, skill: super::skill::Skill) -> Option<ItemSchema> {
        let items = self.providing_exp(level, skill)?;
        items
            .iter()
            .min_set_by_key(|i| self.base_mats_buy_price(&i.code))
            .into_iter()
            .max_by_key(|i| i.level)
            .cloned()
    }

    pub fn providing_exp(&self, level: i32, skill: super::skill::Skill) -> Option<Vec<ItemSchema>> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.api
            .all(
                Some(min),
                Some(level),
                None,
                None,
                Some(&skill.to_string()),
                None,
            )
            .ok()
    }

    pub fn lowest_providing_exp(
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
                None,
                None,
                Some(&skill.to_string()),
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

    pub fn highest_providing_exp(
        &self,
        level: i32,
        skill: super::skill::Skill,
    ) -> Option<Vec<ItemSchema>> {
        let items = self
            .api
            .all(
                None,
                Some(level),
                None,
                None,
                Some(&skill.to_string()),
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

    pub fn craft_schema(&self, code: &str) -> Option<Box<CraftSchema>> {
        self.api.info(code).ok()?.data.item.craft?
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

    pub fn with_material(&self, code: &str) -> Option<Vec<ItemSchema>> {
        self.api.all(None, None, None, None, None, Some(code)).ok()
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
        if let Ok(info) = self.api.info(code) {
            if info.data.item.subtype == "mob" {
                if let Some(monsters) = self.monsters.dropping(code) {
                    rate = monsters
                        .into_iter()
                        .map(|m| {
                            m.drops
                                .into_iter()
                                .find(|d| d.code == code)
                                .map(|d| d.rate)
                                .unwrap_or(0)
                        })
                        .min()
                        .unwrap_or(0)
                }
            } else if let Some(resources) = self.resources.dropping(code) {
                rate = resources
                    .into_iter()
                    .map(|m| {
                        m.drops
                            .into_iter()
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
        self.api.info(code).ok()?.data.item.effects
    }

    pub fn damages(&self, code: &str) -> i32 {
        self.effects_of(code).map_or(0, |e| {
            e.iter()
                .filter(|e| !e.name.starts_with("attack_"))
                .map(|e| e.value)
                .sum()
        })
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
