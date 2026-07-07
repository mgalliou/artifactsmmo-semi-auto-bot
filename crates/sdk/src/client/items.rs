use crate::{
    Cached, Code, CollectionClient, DropsItems, Level, Quantity,
    client::{
        monsters::MonstersClient, npcs::NpcsClient, resources::ResourcesClient,
        tasks_rewards::TasksRewardsClient,
    },
    consts::{TASKS_COIN, TASKS_REWARDS_SPECIFICS},
    entities::{Item, Monster, Npc, Resource},
    gear::Slot,
    simulator::{EffectCode, HasEffects},
    skill::Skill,
};
use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use openapi::models::SimpleItemSchema;
use std::{collections::HashMap, fmt, sync::Arc, vec::Vec};
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Clone, Default, Deref, CollectionClient)]
#[deref(forward)]
#[element(Item)]
pub struct ItemsClient(Arc<ItemsClientInner>);

pub struct ItemsClientInner {
    path: Box<str>,
    data: ArcSwap<HashMap<String, Item>>,
    fetch: Box<dyn Fn() -> HashMap<String, Item> + Send + Sync>,
    resources: ResourcesClient,
    monsters: MonstersClient,
    tasks_rewards: TasksRewardsClient,
    npcs: NpcsClient,
}

impl Default for ItemsClientInner {
    fn default() -> Self {
        Self {
            path: Box::from(".cache/items.ron"),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("ItemsClient not initialized")),
            resources: ResourcesClient::default(),
            monsters: MonstersClient::default(),
            tasks_rewards: TasksRewardsClient::default(),
            npcs: NpcsClient::default(),
        }
    }
}

impl ItemsClient {
    pub(crate) fn new(
        path: &str,
        fetch: Box<dyn Fn() -> HashMap<String, Item> + Send + Sync>,
        resources: ResourcesClient,
        monsters: MonstersClient,
        tasks_rewards: TasksRewardsClient,
        npcs: NpcsClient,
    ) -> Self {
        Self(
            ItemsClientInner {
                path: path.into(),
                fetch,
                data: ArcSwap::default(),
                resources,
                monsters,
                tasks_rewards,
                npcs,
            }
            .into(),
        )
    }

    #[must_use]
    pub fn from_cache(path: &str) -> Self {
        let client = Self::new(
            path,
            Box::new(|| unreachable!("ItemsClient::from_cache has no API fallback")),
            ResourcesClient::default(),
            MonstersClient::default(),
            TasksRewardsClient::default(),
            NpcsClient::default(),
        );
        client.init();
        client
    }

    pub fn init(&self) {
        self.data.store(Arc::new(self.fetch()));
        info!("Items client initilized");
    }

    /// Takes an item `code` and return the mats required to craft it.
    pub fn mats_of(&self, code: &str) -> Vec<SimpleItemSchema> {
        self.get(code).iter().flat_map(Item::mats).collect_vec()
    }

    #[must_use]
    pub fn mats_for(&self, code: &str, quantity: u32) -> Vec<SimpleItemSchema> {
        self.mats_of(code)
            .into_iter()
            .update(|m| m.quantity *= quantity)
            .collect_vec()
    }

    /// Takes an item `code` and returns the mats down to the raw materials
    /// required to craft it.
    #[must_use]
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
    #[must_use]
    pub fn crafted_from_resource(&self, resource_code: &str) -> Vec<Item> {
        self.resources
            .get(resource_code)
            .iter()
            .flat_map(|r| {
                r.drops()
                    .iter()
                    .map(|drop| self.crafted_with_base_mat(&drop.code))
                    .collect_vec()
            })
            .flatten()
            .collect_vec()
    }

    /// Takes an item `code` and returns the items directly crafted with it.
    #[must_use]
    pub fn crafted_with(&self, code: &str) -> Vec<Item> {
        self.iter()
            .filter(|i| i.is_crafted_with(code))
            .collect_vec()
    }

    #[must_use]
    pub fn require_task_reward(&self, code: &str) -> bool {
        self.base_mats_of(code)
            .iter()
            .any(|m| TASKS_REWARDS_SPECIFICS.contains(&m.code.as_str()))
    }

    /// Takes an item `code` and returns the only item it can be crafted in, or
    /// `None` otherwise.
    #[must_use]
    pub fn unique_craft(&self, code: &str) -> Option<Item> {
        let crafts = self.crafted_with(code);
        (crafts.len() == 1)
            .then(|| crafts.first().cloned())
            .flatten()
    }

    /// Takes an item `code` and returns the items crafted with it as base mat.
    #[must_use]
    pub fn crafted_with_base_mat(&self, code: &str) -> Vec<Item> {
        self.iter()
            .filter(|i| self.is_crafted_with_base_mat(i.code(), code))
            .collect_vec()
    }

    /// Takes an item `code` and checks if it is crafted with `mat` as a base
    /// material.
    #[must_use]
    pub fn is_crafted_with_base_mat(&self, code: &str, mat: &str) -> bool {
        self.base_mats_of(code).iter().any(|m| m.code == mat)
    }

    pub fn mats_mob_average_lvl(&self, code: &str) -> u32 {
        let mob_mats = self
            .mats_of(code)
            .iter()
            .filter_map(|i| self.get(&i.code).filter(|i| i.subtype_is(SubType::Mob)))
            .collect_vec();
        let len = mob_mats.len() as u32;
        if len < 1 {
            return 0;
        }
        mob_mats.iter().map(Level::level).sum::<u32>() / len
    }

    pub fn mats_mob_max_lvl(&self, code: &str) -> u32 {
        self.mats_of(code)
            .iter()
            .filter_map(|i| self.get(&i.code).filter(|i| i.subtype_is(SubType::Mob)))
            .max_by_key(Level::level)
            .map_or(0, |i| i.level())
    }

    /// Takes an item `code` and returns the amount of inventory space the mats
    /// required to craft it are taking.
    #[must_use]
    pub fn mats_quantity_for(&self, code: &str) -> u32 {
        self.mats_of(code).iter().map(Quantity::quantity).sum()
    }

    #[must_use]
    pub fn recycled_quantity_for(&self, code: &str) -> u32 {
        let mats_quantity_for = self.mats_quantity_for(code);
        mats_quantity_for / 5 + u32::from(!mats_quantity_for.is_multiple_of(5))
    }

    #[must_use]
    pub fn restoring_utilities(&self, level: u32) -> Vec<Item> {
        self.iter()
            .filter(|i| i.r#type().is_utility() && i.restore() > 0 && i.level() >= level)
            .collect_vec()
    }

    #[must_use]
    pub fn upgrades_of(&self, item_code: &str) -> Vec<Item> {
        let Some(item) = self.get(item_code) else {
            return vec![];
        };
        self.iter()
            .filter(|i| {
                i.code() != item.code()
                    && i.type_is(item.r#type())
                    && item.effects().iter().all(|e| {
                        if e.code == EffectCode::InventorySpace
                            || e.code == EffectCode::Mining
                            || e.code == EffectCode::Woodcutting
                            || e.code == EffectCode::Fishing
                            || e.code == EffectCode::Alchemy
                        {
                            e.value >= i.effect_value(&e.code)
                        } else {
                            e.value <= i.effect_value(&e.code)
                        }
                    })
            })
            .collect_vec()
    }

    pub fn sources_of(&self, code: &str) -> Vec<ItemSource> {
        if code == TASKS_COIN {
            return vec![ItemSource::Task];
        }
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
        if self.get(code).is_some_and(|i| i.is_craftable()) {
            sources.push(ItemSource::Craft);
        }
        if self.tasks_rewards.any(|r| r.code() == code) {
            sources.push(ItemSource::TaskReward);
        }
        sources.extend(
            self.npcs
                .selling(code)
                .into_iter()
                .map(ItemSource::Npc)
                .collect_vec(),
        );
        sources
    }

    #[must_use]
    pub fn is_from_event(&self, code: &str) -> bool {
        self.get(code).is_some_and(|i| {
            self.sources_of(i.code()).iter().any(|s| match s {
                ItemSource::Resource(resource) => self.resources.is_event(resource.code()),
                ItemSource::Monster(monster) => self.monsters.is_event(monster.code()),
                ItemSource::Npc(npc) => npc.is_merchant(),
                ItemSource::Craft | ItemSource::TaskReward | ItemSource::Task => false,
            })
        })
    }

    #[must_use]
    pub fn is_buyable(&self, item_code: &str) -> bool {
        self.npcs
            .items()
            .get(item_code)
            .is_some_and(|i| i.is_buyable())
    }

    #[must_use]
    pub fn is_salable(&self, item_code: &str) -> bool {
        self.npcs
            .items()
            .get(item_code)
            .is_some_and(|i| i.is_salable())
    }
}

impl Cached<HashMap<String, Item>> for ItemsClient {
    fn path(&self) -> &str {
        &self.path
    }

    fn fetch_from_source(&self) -> HashMap<String, Item> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
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
    Bag,
    Rune,
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
            Slot::Ring1 | Slot::Ring2 => Self::Ring,
            Slot::Amulet => Self::Amulet,
            Slot::Artifact1 | Slot::Artifact2 | Slot::Artifact3 => Self::Artifact,
            Slot::Utility1 | Slot::Utility2 => Self::Utility,
            Slot::Bag => Self::Bag,
            Slot::Rune => Self::Rune,
        }
    }
}

impl PartialEq<Type> for String {
    fn eq(&self, other: &Type) -> bool {
        other.as_ref() == *self
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
#[strum(serialize_all = "snake_case")]
pub enum SubType {
    Alchemy,
    Alloy,
    Bar,
    Bag,
    Fishing,
    Food,
    Mining,
    Mob,
    Npc,
    Potion,
    Sap,
    Plank,
    Tool,
    Task,
    PreciousStone,
    Woodcutting,
}

impl PartialEq<SubType> for String {
    fn eq(&self, other: &SubType) -> bool {
        other.as_ref() == *self
    }
}

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum ItemSource {
    Resource(Resource),
    Monster(Monster),
    Npc(Npc),
    Craft,
    TaskReward,
    Task,
}

impl fmt::Display for ItemSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resource(r) => {
                write!(f, "Resource ({})", r.name())
            }
            Self::Monster(m) => write!(f, "Monster ({})", m.name()),
            Self::Npc(npc_schema) => write!(f, "NPC ({})", npc_schema.name()),
            Self::Craft => write!(f, "Craft"),
            Self::TaskReward => write!(f, "Task Reward"),
            Self::Task => write!(f, "Task"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
#[strum(serialize_all = "snake_case")]
pub enum LevelConditionCode {
    AlchemyLevel,
    MiningLevel,
    WoodcuttingLevel,
    FishingLevel,
    Level,
}

impl From<LevelConditionCode> for Skill {
    fn from(value: LevelConditionCode) -> Self {
        match value {
            LevelConditionCode::AlchemyLevel => Self::Alchemy,
            LevelConditionCode::MiningLevel => Self::Mining,
            LevelConditionCode::WoodcuttingLevel => Self::Woodcutting,
            LevelConditionCode::FishingLevel => Self::Fishing,
            LevelConditionCode::Level => Self::Combat,
        }
    }
}

impl PartialEq<LevelConditionCode> for String {
    fn eq(&self, other: &LevelConditionCode) -> bool {
        other.as_ref() == *self
    }
}

#[cfg(test)]
mod tests {
    use crate::simulator::{DamageType, HasEffects};
    use crate::test_utils::{ITEMS, item, monster};

    #[test]
    fn item_damage_against() {
        let val = item("skull_staff").average_dmg_against(&monster("ogre"));
        assert!(val > 0.0, "skull_staff vs ogre dmg = {val}");

        let val = item("dreadful_staff").average_dmg_against(&monster("vampire"));
        assert!(val > 0.0, "dreadful_staff vs vampire dmg = {val}");
    }

    #[test]
    fn damage_increase() {
        assert_eq!(item("steel_boots").dmg_increase(DamageType::Air), 0);
    }

    #[test]
    fn damage_increase_against() {
        let val = item("steel_armor")
            .average_dmg_boost_against_with(&monster("chicken"), &item("steel_battleaxe"));
        assert!(
            val >= 0.0,
            "steel_armor boost vs chicken with battleaxe = {val}"
        );

        let val = item("steel_boots")
            .average_dmg_boost_against_with(&monster("ogre"), &item("skull_staff"));
        assert!(val.abs() < 0.000_1);
    }

    #[test]
    fn damage_reduction_against() {
        let val = item("steel_armor").average_dmg_reduction_against(&monster("ogre"));
        assert!(val > 0.0, "steel_armor reduction vs ogre = {val}");
    }

    #[test]
    fn require_task_reward() {
        assert!(ITEMS.require_task_reward("greater_dreadful_staff"));
    }

    #[test]
    fn mats_methods() {
        assert!(!ITEMS.mats_of("greater_dreadful_staff").is_empty());
        assert!(!ITEMS.base_mats_of("greater_dreadful_staff").is_empty());
    }
}
