use super::consts::{
    ASTRALYTE_CRYSTAL, DIAMOND, ENCHANTED_FABRIC, FOOD_BLACK_LIST, JASPER_CRYSTAL, MAGICAL_CURE,
    TASKS_COIN,
};
use super::fight_simulator::FightSimulator;
use super::game_config::GameConfig;
use super::gear::Slot;
use super::monsters::MonsterSchemaExt;
use super::resources::ResourceSchemaExt;
use super::skill::Skill;
use super::tasks::Tasks;
use super::{api::items::ItemsApi, monsters::Monsters, resources::Resources};
use super::{persist_data, retreive_data};
use artifactsmmo_openapi::models::{CraftSchema, ItemEffectSchema, ItemSchema, SimpleItemSchema};
use artifactsmmo_openapi::models::{MonsterSchema, ResourceSchema};
use itertools::Itertools;
use log::{debug, error};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::{sync::Arc, vec::Vec};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Default)]
pub struct Items {
    pub data: HashMap<String, ItemSchema>,
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
        let data = if let Ok(data) = retreive_data(path) {
            data
        } else {
            let data = api
                .all(None, None, None, None, None, None)
                .expect("items to be retrieved from API.")
                .into_iter()
                .map(|item| (item.code.clone(), item))
                .collect();
            if let Err(e) = persist_data(&data, path) {
                error!("failed to persist items data: {}", e);
            }
            data
        };
        Items {
            data,
            resources: resources.clone(),
            monsters: monsters.clone(),
            tasks: tasks.clone(),
        }
    }

    /// Takes an item `code` and return its schema.
    pub fn get(&self, code: &str) -> Option<&ItemSchema> {
        self.data.get(code)
    }

    /// Takes an item `code` and return the mats required to craft it.
    pub fn mats_of(&self, code: &str) -> Vec<SimpleItemSchema> {
        self.get(code).iter().flat_map(|i| i.mats()).collect_vec()
    }

    /// Takes an item `code` and returns the mats down to the raw materials
    /// required to craft it.
    pub fn base_mats_of(&self, code: &str) -> Vec<SimpleItemSchema> {
        self.mats_of(code)
            .iter()
            .flat_map(|mat| {
                if self.mats_of(&mat.code).is_empty() {
                    vec![SimpleItemSchema {
                        code: mat.code.clone(),
                        quantity: mat.quantity,
                    }]
                } else {
                    self.mats_of(&mat.code)
                        .iter()
                        .map(|b| SimpleItemSchema {
                            code: b.code.clone(),
                            quantity: b.quantity * mat.quantity,
                        })
                        .collect_vec()
                }
            })
            .collect_vec()
    }

    /// Takes an `resource` code and returns the items that can be crafted
    /// from the base mats it drops.
    pub fn crafted_from_resource(&self, resource: &str) -> Vec<&ItemSchema> {
        self.resources
            .get(resource)
            .map(|r| &r.drops)
            .into_iter()
            .flatten()
            .flat_map(|i| self.crafted_with_base_mat(&i.code))
            .collect_vec()
    }

    /// Takes an item `code` and returns the items directly crafted with it.
    pub fn crafted_with(&self, code: &str) -> Vec<&ItemSchema> {
        self.data
            .values()
            .filter(|i| i.is_crafted_with(code))
            .collect_vec()
    }

    pub fn require_task_reward(&self, code: &str) -> bool {
        self.base_mats_of(code).iter().any(|m| {
            [
                JASPER_CRYSTAL,
                MAGICAL_CURE,
                ENCHANTED_FABRIC,
                ASTRALYTE_CRYSTAL,
            ]
            .contains(&m.code.as_str())
        })
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
            .values()
            .filter(|i| self.is_crafted_with_base_mat(&i.code, code))
            .collect_vec()
    }

    /// Takes an item `code` and checks if it is crafted with `mat` as a base
    /// material.
    pub fn is_crafted_with_base_mat(&self, code: &str, mat: &str) -> bool {
        self.base_mats_of(code).iter().any(|m| m.code == mat)
    }

    pub fn mats_mob_average_lvl(&self, code: &str) -> i32 {
        let mob_mats = self
            .mats_of(code)
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
        self.mats_of(code)
            .iter()
            .filter_map(|i| self.get(&i.code))
            .filter(|i| i.subtype == SubType::Mob)
            .max_by_key(|i| i.level)
            .map_or(0, |i| i.level)
    }

    /// Takes an item `code` and returns the amount of inventory space the mats
    /// required to craft it are taking.
    pub fn mats_quantity_for(&self, code: &str) -> i32 {
        self.mats_of(code).iter().map(|mat| mat.quantity).sum()
    }

    /// Takes an item `code` and returns the best (lowest value) drop rate from
    /// `Monsters` or `Resources`
    pub fn drop_rate(&self, code: &str) -> i32 {
        self.monsters
            .dropping(code)
            .iter()
            .flat_map(|m| &m.drops)
            .chain(self.resources.dropping(code).iter().flat_map(|m| &m.drops))
            .find(|d| d.code == code)
            .map_or(0, |d| {
                (d.rate as f32 * ((d.min_quantity + d.max_quantity) as f32 / 2.0)).round() as i32
            })
    }

    /// Takes an item `code` and aggregate the drop rates of its base materials
    /// to cumpute an average drop rate.
    pub fn base_mats_drop_rate(&self, code: &str) -> f32 {
        let base_mats = self.base_mats_of(code);
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
            .values()
            .filter(|i| i.r#type == r#type && i.level <= level)
            .collect_vec()
    }

    pub fn best_consumable_foods(&self, level: i32) -> Vec<&ItemSchema> {
        self.data
            .values()
            .filter(|i| i.is_consumable(level))
            .collect_vec()
    }

    pub fn restoring_utilities(&self, level: i32) -> Vec<&ItemSchema> {
        self.data
            .values()
            .filter(|i| i.r#type().is_utility() && i.restore() > 0 && i.level >= level)
            .collect_vec()
    }

    /// Takes a `level` and a item `code` and returns all the items of the same
    /// type for which the level is between the given `level` and the item level.
    pub fn potential_upgrade(&self, level: i32, code: &str) -> Vec<&ItemSchema> {
        self.data
            .values()
            .filter(|u| {
                self.get(code)
                    .is_some_and(|i| u.r#type == i.r#type && u.level >= i.level)
                    && u.level <= level
            })
            .collect_vec()
    }

    /// NOTE: WIP: there is a lot of edge cases here:
    /// if all sources are resources or monsters, then the lowest drop rate source should be returned,
    /// if the drop rate sources is the same for all sources (algea), either the sources also
    /// containing other item ordereds should be returned, otherwise the one with the higest(lowest
    /// for speed?) level, or time to kill
    /// (or archivment maybe).
    /// All this logic should probably be done elsewhere since it can be related to the orderboard
    /// or the character level/skill_level/gear.
    pub fn best_source_of(&self, code: &str) -> Option<ItemSource> {
        if code == "gift" {
            return Some(ItemSource::Monster(
                self.monsters.get("gingerbread").unwrap(),
            ));
        }
        let sources = self.sources_of(code);
        if sources.iter().all(|s| s.is_resource() || s.is_monster()) {
            let bests = self.sources_of(code).into_iter().min_set_by_key(|s| {
                if let ItemSource::Resource(r) = s {
                    r.drop_rate(code)
                } else if let ItemSource::Monster(m) = s {
                    m.drop_rate(code)
                } else {
                    None
                }
            });
            bests.first().cloned()
        } else {
            sources.first().cloned()
        }
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
        if code == TASKS_COIN {
            sources.push(ItemSource::Task);
        }
        if [
            "blue_candy",
            "green_candy",
            "red_candy",
            "yellow_candy",
            "christmas_cane",
            "christmas_star",
            "frozen_gloves",
            "frozen_axe",
            "frozen_fishing_rode",
            "frozen_pickaxe",
        ]
        .contains(&code)
        {
            sources.push(ItemSource::Gift);
        }
        sources
    }

    pub fn time_to_get(&self, item: &str) -> Option<i32> {
        self.sources_of(item)
            .iter()
            .map(|s| match s {
                ItemSource::Resource(_) => 20,
                ItemSource::Monster(m) => m.level * self.drop_rate(item),
                ItemSource::Craft => self
                    .mats_of(item)
                    .iter()
                    .map(|m| self.time_to_get(&m.code).unwrap_or(10000) * m.quantity)
                    .sum(),
                ItemSource::TaskReward => 20000,
                ItemSource::Task => 20000,
                ItemSource::Gift => 10000,
            })
            .min()
    }

    pub fn is_from_event(&self, code: &str) -> bool {
        self.get(code).map_or(false, |i| {
            self.sources_of(&i.code).iter().any(|s| match s {
                ItemSource::Resource(r) => self.resources.is_event(&r.code),
                ItemSource::Monster(m) => self.monsters.is_event(&m.code),
                ItemSource::Craft => false,
                ItemSource::TaskReward => false,
                ItemSource::Task => false,
                ItemSource::Gift => false,
            })
        })
    }
}

pub trait ItemSchemaExt {
    fn name(&self) -> String;
    fn r#type(&self) -> Type;
    fn is_of_type(&self, r#type: Type) -> bool;
    fn is_crafted_with(&self, item: &str) -> bool;
    fn is_crafted_from_task(&self) -> bool;
    fn mats(&self) -> Vec<SimpleItemSchema>;
    fn craft_schema(&self) -> Option<CraftSchema>;
    fn skill_to_craft(&self) -> Option<Skill>;
    fn effects(&self) -> Vec<&ItemEffectSchema>;
    fn attack_damage(&self, r#type: DamageType) -> i32;
    fn attack_damage_against(&self, monster: &MonsterSchema) -> f32;
    fn damage_increase(&self, r#type: DamageType) -> i32;
    fn resistance(&self, r#type: DamageType) -> i32;
    fn health(&self) -> i32;
    fn haste(&self) -> i32;
    fn skill_cooldown_reduction(&self, skijll: Skill) -> i32;
    fn heal(&self) -> i32;
    fn restore(&self) -> i32;
    fn inventory_space(&self) -> i32;
    fn is_consumable(&self, level: i32) -> bool;
    fn damage_increase_against_with(&self, monster: &MonsterSchema, weapon: &ItemSchema) -> f32;
    fn damage_reduction_against(&self, monster: &MonsterSchema) -> f32;
}

impl ItemSchemaExt for ItemSchema {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn r#type(&self) -> Type {
        Type::from_str(&self.r#type).expect("type to be valid")
    }

    fn is_of_type(&self, r#type: Type) -> bool {
        self.r#type == r#type
    }

    fn is_crafted_with(&self, item: &str) -> bool {
        self.mats().iter().any(|m| m.code == item)
    }

    fn is_crafted_from_task(&self) -> bool {
        self.is_crafted_with(JASPER_CRYSTAL)
            || self.is_crafted_with(MAGICAL_CURE)
            || self.is_crafted_with(ENCHANTED_FABRIC)
            || self.is_crafted_with(ASTRALYTE_CRYSTAL)
            || self.is_crafted_with(DIAMOND)
            || self.is_crafted_with("rosenblood_elixir")
            || self.is_crafted_with("hellhound_hair")
            || self.is_crafted_with("efreet_cloth")
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

    fn attack_damage_against(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| FightSimulator::average_dmg(self.attack_damage(t), 0, monster.resistance(t)))
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

    fn heal(&self) -> i32 {
        self.effects()
            .iter()
            .find_map(|e| (e.name == "heal").then_some(e.value))
            .unwrap_or(0)
    }

    fn restore(&self) -> i32 {
        self.effects()
            .iter()
            .find_map(|e| (e.name == "restore").then_some(e.value))
            .unwrap_or(0)
    }

    fn inventory_space(&self) -> i32 {
        self.effects()
            .iter()
            .find_map(|e| (e.name == "inventory_space").then_some(e.value))
            .unwrap_or(0)
    }

    fn is_consumable(&self, level: i32) -> bool {
        self.is_of_type(Type::Consumable)
            && self.heal() > 0
            && self.level <= level
            && !FOOD_BLACK_LIST.contains(&self.code.as_str())
    }

    fn damage_increase_against_with(&self, monster: &MonsterSchema, weapon: &ItemSchema) -> f32 {
        DamageType::iter()
            .map(|t| {
                FightSimulator::average_dmg(
                    weapon.attack_damage(t),
                    self.damage_increase(t),
                    monster.resistance(t),
                )
            })
            .sum::<f32>()
            - DamageType::iter()
                .map(|t| {
                    FightSimulator::average_dmg(weapon.attack_damage(t), 0, monster.resistance(t))
                })
                .sum::<f32>()
    }

    fn damage_reduction_against(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| FightSimulator::average_dmg(monster.attack_damage(t), 0, 0))
            .sum::<f32>()
            - DamageType::iter()
                .map(|t| {
                    FightSimulator::average_dmg(monster.attack_damage(t), 0, self.resistance(t))
                })
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

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum ItemSource<'a> {
    Resource(&'a ResourceSchema),
    Monster(&'a MonsterSchema),
    Craft,
    TaskReward,
    Task,
    Gift,
}

#[cfg(test)]
mod tests {
    use super::Items;
    use crate::artifactsmmo_sdk::items::ItemSchemaExt;
    use crate::artifactsmmo_sdk::{
        game_config::GameConfig, items::ItemSource, monsters::Monsters, resources::Resources,
        tasks::Tasks,
    };
    use itertools::Itertools;
    use std::sync::Arc;

    #[test]
    fn potential_upgrade() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
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
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
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

    #[test]
    fn damage_increase() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

        assert_eq!(
            items
                .get("steel_boots")
                .unwrap()
                .damage_increase(super::DamageType::Air),
            0
        )
    }

    #[test]
    fn damage_increase_against() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

        assert_eq!(
            items
                .get("steel_armor")
                .unwrap()
                .damage_increase_against_with(
                    monsters.get("chicken").unwrap(),
                    items.get("steel_battleaxe").unwrap()
                ),
            6.0
        );

        assert_eq!(
            items
                .get("steel_boots")
                .unwrap()
                .damage_increase_against_with(
                    monsters.get("ogre").unwrap(),
                    items.get("skull_staff").unwrap()
                ),
            0.0
        );
    }

    #[test]
    fn damage_reduction_against() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

        assert_eq!(
            items
                .get("steel_armor")
                .unwrap()
                .damage_reduction_against(monsters.get("ogre").unwrap()),
            4.0
        );
    }

    #[test]
    fn gift_source() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

        assert_eq!(
            items.sources_of("christmas_star").first(),
            Some(&ItemSource::Gift)
        );
        assert_eq!(
            items.best_source_of("gift"),
            Some(&ItemSource::Monster(monsters.get("gingerbread").unwrap())).cloned()
        );
    }

    #[test]
    fn best_consumable_foods() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

        assert_eq!(
            items
                .best_consumable_foods(29)
                .iter()
                .max_by_key(|i| i.heal())
                .unwrap()
                .code,
            "cooked_trout"
        );
    }

    #[test]
    fn drop_rate() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));
        assert_eq!(items.drop_rate("milk_bucket"), 12);
    }

    #[test]
    fn require_task_reward() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

        assert!(items.require_task_reward("greater_dreadful_staff"));
    }

    #[test]
    fn mats_methods() {
        let config = GameConfig::from_file();
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));

        assert!(!items.mats_of("greater_dreadful_staff").is_empty());
        assert!(!items.base_mats_of("greater_dreadful_staff").is_empty());
    }
}
