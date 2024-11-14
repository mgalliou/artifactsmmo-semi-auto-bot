use super::game_config::GameConfig;
use super::gear::Slot;
use super::skill::Skill;
use super::tasks::Tasks;
use super::{api::items::ItemsApi, monsters::Monsters, resources::Resources};
use super::{average_dmg, persist_data, retreive_data, ItemSchemaExt, MonsterSchemaExt};
use artifactsmmo_openapi::models::{CraftSchema, ItemEffectSchema, ItemSchema, SimpleItemSchema};
use artifactsmmo_openapi::models::{MonsterSchema, ResourceSchema};
use itertools::Itertools;
use log::{debug, error};
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::{sync::Arc, vec::Vec};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Default)]
pub struct Items {
    pub data: Vec<ItemSchema>,
    pub api: ItemsApi,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
    tasks: Arc<Tasks>,
}

impl Items {
    pub fn new(
        config: &GameConfig,
        resources: &Arc<Resources>,
        monsters: &Arc<Monsters>,
        tasks: &Arc<Tasks>,
    ) -> Items {
        let api = ItemsApi::new(&config.base_url);
        let path = Path::new(".cache/items.json");
        let data = if let Ok(data) = retreive_data::<Vec<ItemSchema>>(path) {
            data
        } else {
            let data = api
                .all(None, None, None, None, None, None)
                .expect("items to be retrieved from API.");
            if let Err(e) = persist_data(&data, path) {
                error!("failed to persist items data: {}", e);
            }
            data
        };
        Items {
            data,
            api,
            resources: resources.clone(),
            monsters: monsters.clone(),
            tasks: tasks.clone(),
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

    /// Takes an item `code` and returns the only item it can be crafted in, or
    /// `None` otherwise.
    pub fn unique_craft(&self, code: &str) -> Option<&ItemSchema> {
        let crafts = self.crafted_with(code);
        if crafts.len() == 1 {
            return Some(crafts[0]);
        }
        None
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

    pub fn mats_mob_average_lvl(&self, code: &str) -> i32 {
        let mob_mats = self
            .mats(code)
            .iter()
            .filter_map(|i| self.get(&i.code))
            .filter(|i| i.subtype == SubType::Mob)
            .collect_vec();
        let len = mob_mats.len();
        if len > 0 {
            return mob_mats.iter().map(|i| i.level).sum::<i32>() / mob_mats.len() as i32;
        }
        0
    }

    pub fn mats_mob_max_lvl(&self, code: &str) -> i32 {
        self.mats(code)
            .iter()
            .filter_map(|i| self.get(&i.code))
            .filter(|i| i.subtype == SubType::Mob)
            .max_by_key(|i| i.level)
            .map_or(0, |i| i.level)
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

    pub fn equipable_at_level(&self, level: i32, r#type: Type) -> Vec<&ItemSchema> {
        self.data
            .iter()
            .filter(|i| i.r#type == r#type && i.level <= level)
            .collect_vec()
    }

    /// Returns the best items to level the given `skill` at the given `level.
    pub fn best_for_leveling_hc(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        match skill {
            Skill::Gearcrafting => {
                if level >= 20 {
                    return self.best_for_leveling(level, skill);
                } else if level >= 10 {
                    vec![self.get("iron_helm")]
                } else if level >= 5 {
                    vec![self.get("copper_legs_armor")]
                } else {
                    vec![self.get("wooden_shield")]
                }
            }
            Skill::Weaponcrafting => {
                if level >= 20 {
                    return self.best_for_leveling(level, skill);
                } else if level >= 10 {
                    vec![self.get("iron_dagger")]
                } else {
                    vec![self.get("copper_dagger")]
                }
            }
            Skill::Jewelrycrafting => {
                if level >= 30 {
                    vec![self.get("gold_ring")]
                } else if level >= 25 {
                    vec![self.get("steel_ring")]
                } else if level >= 20 {
                    vec![self.get("life_ring")]
                } else if level >= 10 {
                    vec![self.get("iron_ring")]
                } else {
                    vec![self.get("copper_ring")]
                }
            }
            Skill::Cooking => {
                if level >= 30 {
                    vec![self.get("cooked_bass")]
                } else if level >= 20 {
                    vec![self.get("cooked_trout")]
                } else if level >= 10 {
                    vec![self.get("cooked_shrimp")]
                } else {
                    vec![self.get("cooked_gudgeon")]
                }
            }
            Skill::Mining | Skill::Woodcutting | Skill::Alchemy => {
                return self.best_for_leveling(level, skill)
            }
            Skill::Fishing => vec![None],
            Skill::Combat => vec![None],
        }
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn best_for_leveling(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        self.providing_exp(level, skill)
            .into_iter()
            .filter(|i| {
                i.code != "wooden_staff"
                    && i.code != "life_amulet"
                    && i.code != "feather_coat"
                    && !i.is_crafted_with("jasper_crystal")
                    && !i.is_crafted_with("magical_cure")
            })
            .max_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
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

    pub fn sources_of(&self, code: &str) -> Vec<ItemSource> {
        let mut sources = self
            .resources
            .dropping(code)
            .into_iter()
            .map(ItemSource::Resource)
            .collect_vec();
        sources.extend(
            self.monsters
                .dropping(code)
                .into_iter()
                .map(ItemSource::Monster)
                .collect_vec(),
        );
        if self.get(code).is_some_and(|i| i.craft_schema().is_some()) {
            sources.push(ItemSource::Craft);
        }
        if self.tasks.rewards.iter().any(|r| r.code == code) {
            sources.push(ItemSource::TaskReward);
        }
        if code == "tasks_coin" {
            sources.push(ItemSource::Task);
        }
        sources
    }
}

impl ItemSchemaExt for ItemSchema {
    fn name(&self) -> String {
        self.name.to_owned()
    }

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

    fn total_attack_damage(&self) -> i32 {
        self.effects()
            .iter()
            .filter(|e| e.name.starts_with("attack_"))
            .map(|e| e.value)
            .sum()
    }

    fn attack_damage(&self, r#type: DamageType) -> i32 {
        self.effects()
            .iter()
            .find(|e| e.name == "attack_".to_string() + r#type.as_ref())
            .map(|e| e.value)
            .unwrap_or(0)
    }

    fn resistance(&self, r#type: DamageType) -> i32 {
        self.effects()
            .iter()
            .find(|e| e.name == "res_".to_string() + r#type.as_ref())
            .map(|e| e.value)
            .unwrap_or(0)
    }

    fn total_damage_increase(&self) -> i32 {
        self.effects()
            .iter()
            .filter(|e| e.name.starts_with("dmg_"))
            .map(|e| e.value)
            .sum()
    }

    fn attack_damage_against(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| average_dmg(self.attack_damage(t), 0, monster.resistance(t)))
            .sum()
    }

    fn damage_from(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| average_dmg(monster.attack_damage(t), 0, self.resistance(t)))
            .sum()
    }

    fn damage_increase(&self, r#type: DamageType) -> i32 {
        self.effects()
            .iter()
            .find(|e| {
                e.name == "dmg_".to_string() + r#type.as_ref()
                    || e.name == "boost_dmg_".to_string() + r#type.as_ref()
            })
            .map(|e| e.value)
            .unwrap_or(0)
    }

    fn total_resistance(&self) -> i32 {
        self.effects()
            .iter()
            .filter(|e| e.name.starts_with("res_"))
            .map(|e| e.value)
            .sum()
    }

    fn health(&self) -> i32 {
        self.effects()
            .iter()
            .find(|e| e.name == "hp" || e.name == "boost_hp")
            .map(|e| e.value)
            .unwrap_or(0)
    }

    fn haste(&self) -> i32 {
        self.effects()
            .iter()
            .find(|e| e.name == "haste")
            .map(|e| e.value)
            .unwrap_or(0)
    }

    fn skill_cooldown_reduction(&self, skill: Skill) -> i32 {
        self.effects()
            .iter()
            .find_map(|e| (e.name == skill.as_ref()).then_some(e.value))
            .unwrap_or(0)
    }

    fn damage_increase_against_with(&self, monster: &MonsterSchema, weapon: &ItemSchema) -> f32 {
        DamageType::iter()
            .map(|t| {
                average_dmg(
                    weapon.attack_damage(t),
                    self.damage_increase(t),
                    monster.resistance(t),
                )
            })
            .sum()
    }

    fn damage_reduction_against(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| average_dmg(monster.attack_damage(t), 0, 0))
            .sum::<f32>()
            - DamageType::iter()
                .map(|t| average_dmg(monster.attack_damage(t), 0, self.resistance(t)))
                .sum::<f32>()
    }
}

impl fmt::Display for dyn ItemSchemaExt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
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
    Currency,
    Utility,
}

impl From<Slot> for Type {
    fn from(value: Slot) -> Self {
        match value {
            Slot::Weapon => Self::Weapon,
            Slot::Shield => Self::Shield,
            Slot::Helmet => Self::Helmet,
            Slot::BodyArmor => Self::BodyArmor,
            Slot::LegArmor => Self::LegArmor,
            Slot::Boots => Self::Boots,
            Slot::Ring1 => Self::Ring,
            Slot::Ring2 => Self::Ring,
            Slot::Amulet => Self::Amulet,
            Slot::Artifact1 => Self::Artifact,
            Slot::Artifact2 => Self::Artifact,
            Slot::Artifact3 => Self::Artifact,
            Slot::Utility1 => Self::Utility,
            Slot::Utility2 => Self::Utility,
        }
    }
}

impl PartialEq<Type> for String {
    fn eq(&self, other: &Type) -> bool {
        other.as_ref() == *self
    }
}

#[derive(Debug, PartialEq, AsRefStr, EnumString)]
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

impl PartialEq<SubType> for String {
    fn eq(&self, other: &SubType) -> bool {
        other.as_ref() == *self
    }
}

#[derive(Debug, Copy, Clone, PartialEq, AsRefStr, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum DamageType {
    Air,
    Earth,
    Fire,
    Water,
}

#[derive(EnumIs)]
pub enum ItemSource<'a> {
    Resource(&'a ResourceSchema),
    Monster(&'a MonsterSchema),
    Craft,
    TaskReward,
    Task,
}

#[cfg(test)]
mod tests {
    use crate::artifactsmmo_sdk::{
        game_config::GameConfig, monsters::Monsters, resources::Resources, tasks::Tasks,
        ItemSchemaExt,
    };
    use figment::{
        providers::{Format, Toml},
        Figment,
    };
    use itertools::Itertools;
    use std::sync::Arc;

    use super::Items;

    #[test]
    fn potential_upgrade() {
        let config: GameConfig = Figment::new()
            .merge(Toml::file_exact("ArtifactsMMO.toml"))
            .extract()
            .unwrap();
        let resources = Arc::new(Resources::new(&config));
        let monsters = Arc::new(Monsters::new(&config));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

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

    #[test]
    fn item_damage_against() {
        let config: GameConfig = Figment::new()
            .merge(Toml::file_exact("ArtifactsMMO.toml"))
            .extract()
            .unwrap();
        let resources = Arc::new(Resources::new(&config));
        let monsters = Arc::new(Monsters::new(&config));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

        assert_eq!(
            items
                .get("skull_staff")
                .unwrap()
                .attack_damage_against(monsters.get("ogre").unwrap()),
            48.0
        );
        assert_eq!(
            items
                .get("dreadful_staff")
                .unwrap()
                .attack_damage_against(monsters.get("vampire").unwrap()),
            57.5
        );
    }
}
