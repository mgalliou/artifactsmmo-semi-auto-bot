use super::skill::Skill;
use super::{account::Account, api::items::ItemsApi, monsters::Monsters, resources::Resources};
use artifactsmmo_openapi::models::{
    equip_schema::Slot, CraftSchema, DropRateSchema, GeItemSchema, ItemEffectSchema, ItemSchema,
    SimpleItemSchema,
};
use enum_stringify::EnumStringify;
use itertools::Itertools;
use log::debug;
use std::str::FromStr;
use std::{sync::Arc, vec::Vec};
use strum_macros::{EnumIter, EnumString};

pub struct Items {
    pub data: Vec<ItemSchema>,
    pub api: ItemsApi,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
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

impl Type {
    pub fn from_slot(slot: Slot) -> Self {
        match slot {
            Slot::Weapon => Type::Weapon,
            Slot::Shield => Type::Shield,
            Slot::Helmet => Type::Helmet,
            Slot::BodyArmor => Type::BodyArmor,
            Slot::LegArmor => Type::LegArmor,
            Slot::Boots => Type::Boots,
            Slot::Ring1 => Type::Ring,
            Slot::Ring2 => Type::Ring,
            Slot::Amulet => Type::Amulet,
            Slot::Artifact1 => Type::Artifact,
            Slot::Artifact2 => Type::Artifact,
            Slot::Artifact3 => Type::Artifact,
            Slot::Consumable1 => Type::Consumable,
            Slot::Consumable2 => Type::Consumable,
        }
    }
}

#[derive(Debug, PartialEq, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum SubType {
    Mining,
    Woodcutting,
    Fishing,
    Food,
    Bar,
    Plank,
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

    /// Takes an item `code` and return its schema.
    pub fn get(&self, code: &str) -> Option<&ItemSchema> {
        self.data.iter().find(|m| m.code == code)
    }

    /// Check if an item `code` is a raw meterial.
    pub fn is_raw_mat(&self, code: &str) -> bool {
        if let Some(item) = self.get(code) {
            return item.r#type == "resource"
                && matches!(
                    SubType::from_str(&item.subtype),
                    Ok(SubType::Mining)
                        | Ok(SubType::Woodcutting)
                        | Ok(SubType::Fishing)
                        | Ok(SubType::Food)
                );
        }
        true
    }

    /// Takes an item `code` and return is craft schema.
    // TODO: remove clone() call if possible
    fn craft_schema(&self, code: &str) -> Option<CraftSchema> {
        self.get(code)?.craft.clone()?.map(|c| (*c))
    }

    /// Check if an item `code` is craftable.
    pub fn is_craftable(&self, code: &str) -> bool {
        self.craft_schema(code).is_some()
    }

    /// Takes an item `code` and returns the skill required to craft it.
    pub fn skill_to_craft(&self, code: &str) -> Option<Skill> {
        self.craft_schema(code)
            .and_then(|schema| schema.skill)
            .map(Skill::from_craft_schema_skill)
    }

    /// Takes an item `code` and return the mats required to craft it.
    pub fn mats(&self, code: &str) -> Vec<SimpleItemSchema> {
        self.craft_schema(code)
            .into_iter()
            .filter_map(|i| i.items)
            .flatten()
            .collect_vec()
    }

    /// Takes an item `code` and returns the mats down to the raw materials
    /// required to craft it.
    pub fn base_mats(&self, code: &str) -> Vec<SimpleItemSchema> {
        self.mats(code)
            .iter()
            .flat_map(|mat| {
                self.base_mats(&mat.code)
                    .iter()
                    .map(|b| SimpleItemSchema {
                        code: b.code.clone(),
                        quantity: b.quantity * mat.quantity,
                    })
                    .collect_vec()
            })
            .collect_vec()
    }

    /// Takes an resource `code` and returns the items that can be crafted
    /// from the base mats it drops.
    pub fn crafted_from_resource(&self, code: &str) -> Vec<&ItemSchema> {
        self.resources
            .get(code)
            .map(|r| &r.drops)
            .into_iter()
            .flatten()
            .flat_map(|i| self.crafted_with_base_mat(&i.code))
            .collect_vec()
    }

    /// Takes an item `code` and returns the items directly crafted with it.
    pub fn crafted_with(&self, code: &str) -> Vec<&ItemSchema> {
        self.data
            .iter()
            .filter(|i| self.is_crafted_with(&i.code, code))
            .collect_vec()
    }

    /// Takes an item `code` and returns the items crafted with it as base mat.
    pub fn crafted_with_base_mat(&self, code: &str) -> Vec<&ItemSchema> {
        self.data
            .iter()
            .filter(|i| self.is_crafted_with_base_mat(&i.code, code))
            .collect_vec()
    }

    /// Takes an item `code` and checks if it is directly crafted with `mat`.
    pub fn is_crafted_with(&self, code: &str, mat: &str) -> bool {
        self.mats(code).iter().any(|m| m.code == mat)
    }

    /// Takes an item `code` and checks if it is crafted with `mat` as a base
    /// material.
    pub fn is_crafted_with_base_mat(&self, code: &str, mat: &str) -> bool {
        self.base_mats(code).iter().any(|m| m.code == mat)
    }

    pub fn ge_info(&self, code: &str) -> Option<GeItemSchema> {
        self.api.info(code).ok()?.data.ge?.map(|ge| (*ge))
    }

    /// Takes an item `code` and returns its base mats buy price at the Grand
    /// Exchange.
    pub fn base_mats_buy_price(&self, code: &str) -> i32 {
        let price = self
            .base_mats(code)
            .iter()
            .map(|mat| {
                self.ge_info(&mat.code)
                    .map_or(0, |i| i.buy_price.unwrap_or(0) * mat.quantity)
            })
            .sum();
        debug!("total price for {}: {}", code, price);
        price
    }

    /// Takes an item `code` and returns the amount of inventory space the mats
    /// required to craft it are taking.
    pub fn mats_quantity_for(&self, code: &str) -> i32 {
        self.mats(code).iter().map(|mat| mat.quantity).sum()
    }

    /// Takes an item `code` and returns the best (lowest value) drop rate from
    /// `Monsters` or `Resources`
    pub fn drop_rate(&self, code: &str) -> i32 {
        self.drops(code)
            .iter()
            .find(|d| d.code == code)
            .map_or(0, |d| d.rate)
    }

    /// Takes an item `code` and returns its drops.
    pub fn drops(&self, code: &str) -> Vec<&DropRateSchema> {
        self.get(code)
            .iter()
            .flat_map(|i| {
                if i.subtype == "mob" {
                    return self
                        .monsters
                        .dropping(code)
                        .iter()
                        .flat_map(|m| &m.drops)
                        .collect_vec();
                } else {
                    return self
                        .resources
                        .dropping(code)
                        .iter()
                        .flat_map(|m| &m.drops)
                        .collect_vec();
                }
            })
            .collect_vec()
    }

    /// Takes an item `code` and aggregate the drop rates of its base materials
    /// to cumpute an average drop rate.
    pub fn base_mats_drop_rate(&self, code: &str) -> f32 {
        let mats = self.base_mats(code);
        if mats.is_empty() {
            return 0.0;
        }
        let total_mats: i32 = mats.iter().map(|m| m.quantity).sum();
        debug!("total mats for {}: {}", code, total_mats);
        let sum: i32 = mats
            .iter()
            .map(|m| self.drop_rate(&m.code) * m.quantity)
            .sum();
        debug!("sum for {}: {}", code, sum);
        let average: f32 = sum as f32 / total_mats as f32;
        debug!("average drop rate for {}: {}", code, average);
        average
    }

    /// Takes an item `code` and returns its effects.
    pub fn effects(&self, code: &str) -> Vec<&ItemEffectSchema> {
        self.get(code)
            .iter()
            .filter_map(|i| i.effects.as_ref())
            .flatten()
            .collect_vec()
    }

    /// Takes an item `code` and returns its total damages.
    pub fn damages(&self, code: &str) -> i32 {
        self.effects(code)
            .iter()
            .filter(|e| !e.name.starts_with("attack_"))
            .map(|e| e.value)
            .sum()
    }

    pub fn equipable_at_level(&self, level: i32, slot: Slot) -> Vec<&ItemSchema> {
        self.data
            .iter()
            .filter(|i| i.level <= level)
            .filter(|i| i.r#type == Type::from_slot(slot).to_string())
            .collect_vec()
    }

    /// Takes a `level` and a `skill` and returns the best items to level the
    /// skill based on its meterials drop rate, and value on the Grand Exchange.
    pub fn best_for_leveling(&self, level: i32, skill: Skill) -> Option<&ItemSchema> {
        self.providing_exp(level, skill)
            .into_iter()
            .filter(|i| !self.is_crafted_with(&i.code, "jasper_crystal"))
            .min_set_by_key(|i| (self.base_mats_drop_rate(&i.code) * 100.0) as i32)
            .into_iter()
            .min_set_by_key(|i| self.base_mats_buy_price(&i.code))
            .into_iter()
            .max_by_key(|i| i.level)
    }

    /// Takes a `level` and a `skill` and returns the items providing experince
    /// when crafted.
    pub fn providing_exp(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.data
            .iter()
            .filter(|i| i.level >= min && i.level <= level)
            .filter(|i| self.skill_to_craft(&i.code).is_some_and(|s| s == skill))
            .collect_vec()
    }

    /// Takes a `level` and a `skill` and returns the items of the lowest level
    /// providing experience when crafted.
    pub fn lowest_providing_exp(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        self.providing_exp(level, skill)
            .iter()
            .min_set_by_key(|i| i.level)
            .into_iter()
            .cloned()
            .collect_vec()
    }

    /// Takes a `level` and a `skill` and returns the items of the highest level
    /// providing experience when crafted.
    pub fn highest_providing_exp(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        self.providing_exp(level, skill)
            .iter()
            .max_set_by_key(|i| i.level)
            .into_iter()
            .cloned()
            .collect_vec()
    }
}
