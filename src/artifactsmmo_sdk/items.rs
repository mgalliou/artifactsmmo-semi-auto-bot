use super::skill::Skill;
use super::ItemSchemaExt;
use super::{account::Account, api::items::ItemsApi, monsters::Monsters, resources::Resources};
use artifactsmmo_openapi::models::{
    equip_schema::Slot, CraftSchema, GeItemSchema, ItemEffectSchema, ItemSchema, SimpleItemSchema,
};
use itertools::Itertools;
use log::debug;
use std::str::FromStr;
use std::{sync::Arc, vec::Vec};
use strum_macros::{AsRefStr, EnumIter, EnumString};

pub struct Items {
    pub data: Vec<ItemSchema>,
    pub api: ItemsApi,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
}

#[derive(Debug, PartialEq, AsRefStr, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
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

impl PartialEq<Type> for String {
    fn eq(&self, other: &Type) -> bool {
        other.as_ref() == *self
    }
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
    Mob,
}

impl ItemSchemaExt for ItemSchema {
    fn is_raw_mat(&self) -> bool {
        self.r#type == "resource"
            && matches!(
                SubType::from_str(&self.subtype),
                Ok(SubType::Mining)
                    | Ok(SubType::Woodcutting)
                    | Ok(SubType::Fishing)
                    | Ok(SubType::Food)
            )
    }

    fn is_of_type(&self, r#type: Type) -> bool {
        self.r#type == r#type
    }

    fn is_crafted_with(&self, code: &str) -> bool {
        self.mats().iter().any(|m| m.code == code)
    }

    fn mats(&self) -> Vec<SimpleItemSchema> {
        self.craft_schema()
            .into_iter()
            .filter_map(|i| i.items)
            .flatten()
            .collect_vec()
    }

    fn craft_schema(&self) -> Option<CraftSchema> {
        self.craft.clone()?.map(|c| (*c))
    }

    fn skill_to_craft(&self) -> Option<Skill> {
        self.craft_schema()
            .and_then(|schema| schema.skill)
            .map(Skill::from)
    }

    fn effects(&self) -> Vec<&ItemEffectSchema> {
        self.effects.iter().flatten().collect_vec()
    }

    fn damages(&self) -> i32 {
        self.effects()
            .iter()
            .filter(|e| !e.name.starts_with("attack_"))
            .map(|e| e.value)
            .sum()
    }
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

    /// Takes an item `code` and return its type.
    pub fn r#type(&self, code: &str) -> Option<Type> {
        Type::from_str(&self.get(code)?.r#type).ok()
    }

    /// Checks an item `code` is of a certain `type`.
    pub fn is_of_type(&self, code: &str, r#type: Type) -> bool {
        self.get(code).is_some_and(|i| i.is_of_type(r#type))
    }

    /// Takes an item `code` and returns the skill required to craft it.
    pub fn skill_to_craft(&self, code: &str) -> Option<Skill> {
        self.get(code)?.skill_to_craft()
    }

    /// Takes an item `code` and return the mats required to craft it.
    pub fn mats(&self, code: &str) -> Vec<SimpleItemSchema> {
        self.get(code).iter().flat_map(|i| i.mats()).collect_vec()
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
            .filter(|i| i.is_crafted_with(code))
            .collect_vec()
    }

    /// Takes an item `code` and returns the items crafted with it as base mat.
    pub fn crafted_with_base_mat(&self, code: &str) -> Vec<&ItemSchema> {
        self.data
            .iter()
            .filter(|i| self.is_crafted_with_base_mat(&i.code, code))
            .collect_vec()
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
            .find(|d| d.code == code)
            .map_or(0, |d| d.rate)
    }

    /// Takes an item `code` and aggregate the drop rates of its base materials
    /// to cumpute an average drop rate.
    pub fn base_mats_drop_rate(&self, code: &str) -> f32 {
        let base_mats = self.base_mats(code);
        if base_mats.is_empty() {
            return 0.0;
        }
        let base_mats_quantity: i32 = base_mats.iter().map(|m| m.quantity).sum();
        debug!("total mats for {}: {}", code, base_mats_quantity);
        let drop_rate_sum: i32 = base_mats
            .iter()
            .map(|m| self.drop_rate(&m.code) * m.quantity)
            .sum();
        debug!("sum for {}: {}", code, drop_rate_sum);
        let average: f32 = drop_rate_sum as f32 / base_mats_quantity as f32;
        debug!("average drop rate for {}: {}", code, average);
        average
    }

    pub fn equipable_at_level(&self, level: i32, slot: Slot) -> Vec<&ItemSchema> {
        self.data
            .iter()
            .filter(|i| i.level <= level)
            .filter(|i| i.r#type == Type::from_slot(slot))
            .collect_vec()
    }

    /// Takes a `level` and a `skill` and returns the best items to level the
    /// skill based on its meterials drop rate, and value on the Grand Exchange.
    pub fn best_for_leveling(&self, level: i32, skill: Skill) -> Option<&ItemSchema> {
        self.providing_exp(level, skill)
            .into_iter()
            .filter(|i| !i.is_crafted_with("jasper_crystal") || i.is_crafted_with("magical_cure"))
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
            .filter(|i| i.skill_to_craft().is_some_and(|s| s == skill))
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

    /// Takes a `level` and a item `code` and returns all the items of the same
    /// type for which the level is between the given `level` and the item level.
    pub fn potential_upgrade(&self, level: i32, code: &str) -> Vec<&ItemSchema> {
        self.data
            .iter()
            .filter(|u| {
                self.get(code)
                    .is_some_and(|i| u.r#type == i.r#type && u.level >= i.level)
                    && u.level <= level
            })
            .collect_vec()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use figment::{
        providers::{Format, Toml},
        Figment,
    };
    use itertools::Itertools;

    use crate::artifactsmmo_sdk::{
        account::Account, config::Config, monsters::Monsters, resources::Resources,
    };

    use super::Items;

    #[test]
    fn tests() {
        let config: Config = Figment::new()
            .merge(Toml::file_exact("ArtifactsMMO.toml"))
            .extract()
            .unwrap();
        let account = Account::new(&config.base_url, &config.token);
        let resources = Arc::new(Resources::new(&account));
        let monsters = Arc::new(Monsters::new(&account));
        let items = Arc::new(Items::new(&account, resources.clone(), monsters.clone()));

        assert_eq!(
            items
                .potential_upgrade(10, "copper_armor")
                .iter()
                .map(|i| &i.code)
                .collect_vec(),
            vec![
                "feather_coat",
                "copper_armor",
                "leather_armor",
                "iron_armor",
                "adventurer_vest"
            ]
        )
    }
}
