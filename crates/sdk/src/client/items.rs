use crate::{
    Code, CollectionClient, DataEntity, DropsItems, Level, Persist,
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
use api::ArtifactApi;
use itertools::Itertools;
use openapi::models::SimpleItemSchema;
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
    vec::Vec,
};
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Default, Debug, CollectionClient)]
pub struct ItemsClient {
    data: RwLock<HashMap<String, Item>>,
    api: Arc<ArtifactApi>,
    resources: Arc<ResourcesClient>,
    monsters: Arc<MonstersClient>,
    tasks_rewards: Arc<TasksRewardsClient>,
    npcs: Arc<NpcsClient>,
}

impl ItemsClient {
    pub(crate) fn new(
        api: Arc<ArtifactApi>,
        resources: Arc<ResourcesClient>,
        monsters: Arc<MonstersClient>,
        tasks_rewards: Arc<TasksRewardsClient>,
        npcs: Arc<NpcsClient>,
    ) -> Self {
        let items = Self {
            data: Default::default(),
            api,
            resources,
            monsters,
            tasks_rewards,
            npcs,
        };
        *items.data.write().unwrap() = items.load();
        items
    }

    /// Takes an item `code` and return the mats required to craft it.
    pub fn mats_of(&self, code: &str) -> Vec<SimpleItemSchema> {
        self.get(code).iter().flat_map(|i| i.mats()).collect_vec()
    }

    pub fn mats_for(&self, code: &str, quantity: u32) -> Vec<SimpleItemSchema> {
        self.mats_of(code)
            .into_iter()
            .update(|m| m.quantity *= quantity)
            .collect_vec()
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
    pub fn crafted_with(&self, code: &str) -> Vec<Item> {
        self.filtered(|i| i.is_crafted_with(code))
    }

    pub fn require_task_reward(&self, code: &str) -> bool {
        self.base_mats_of(code)
            .iter()
            .any(|m| TASKS_REWARDS_SPECIFICS.contains(&m.code.as_str()))
    }

    /// Takes an item `code` and returns the only item it can be crafted in, or
    /// `None` otherwise.
    pub fn unique_craft(&self, code: &str) -> Option<Item> {
        let crafts = self.crafted_with(code);
        (crafts.len() == 1)
            .then_some(crafts.first().cloned())
            .flatten()
    }

    /// Takes an item `code` and returns the items crafted with it as base mat.
    pub fn crafted_with_base_mat(&self, code: &str) -> Vec<Item> {
        self.filtered(|i| self.is_crafted_with_base_mat(i.code(), code))
    }

    /// Takes an item `code` and checks if it is crafted with `mat` as a base
    /// material.
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
        mob_mats.iter().map(|i| i.level()).sum::<u32>() / len
    }

    pub fn mats_mob_max_lvl(&self, code: &str) -> u32 {
        self.mats_of(code)
            .iter()
            .filter_map(|i| self.get(&i.code).filter(|i| i.subtype_is(SubType::Mob)))
            .max_by_key(|i| i.level())
            .map_or(0, |i| i.level())
    }

    /// Takes an item `code` and returns the amount of inventory space the mats
    /// required to craft it are taking.
    pub fn mats_quantity_for(&self, code: &str) -> u32 {
        self.mats_of(code).iter().map(|mat| mat.quantity).sum()
    }

    pub fn recycled_quantity_for(&self, code: &str) -> u32 {
        let mats_quantity_for = self.mats_quantity_for(code);
        mats_quantity_for / 5
            + if mats_quantity_for.is_multiple_of(5) {
                0
            } else {
                1
            }
    }

    pub fn restoring_utilities(&self, level: u32) -> Vec<Item> {
        self.filtered(|i| i.r#type().is_utility() && i.restore() > 0 && i.level() >= level)
    }

    pub fn upgrades_of(&self, item_code: &str) -> Vec<Item> {
        let Some(item) = self.get(item_code) else {
            return vec![];
        };
        self.filtered(|i| {
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
        if self.tasks_rewards.all().iter().any(|r| r.code() == code) {
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

    pub fn is_from_event(&self, code: &str) -> bool {
        self.get(code).is_some_and(|i| {
            self.sources_of(i.code()).iter().any(|s| match s {
                ItemSource::Resource(resource) => self.resources.is_event(resource.code()),
                ItemSource::Monster(monster) => self.monsters.is_event(monster.code()),
                ItemSource::Npc(npc) => npc.is_merchant(),
                ItemSource::Craft => false,
                ItemSource::TaskReward => false,
                ItemSource::Task => false,
            })
        })
    }

    pub fn is_buyable(&self, item_code: &str) -> bool {
        self.npcs
            .items
            .get(item_code)
            .is_some_and(|i| i.is_buyable())
    }

    pub fn is_salable(&self, item_code: &str) -> bool {
        self.npcs
            .items
            .get(item_code)
            .is_some_and(|i| i.is_salable())
    }
}

impl Persist<HashMap<String, Item>> for ItemsClient {
    const PATH: &'static str = ".cache/items.json";

    fn load_from_api(&self) -> HashMap<String, Item> {
        self.api
            .items
            .get_all()
            .unwrap()
            .into_iter()
            .map(|i| (i.code.clone(), Item::new(i)))
            .collect()
    }

    fn refresh(&self) {
        *self.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for ItemsClient {
    type Entity = Item;
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
            Slot::Ring1 => Self::Ring,
            Slot::Ring2 => Self::Ring,
            Slot::Amulet => Self::Amulet,
            Slot::Artifact1 => Self::Artifact,
            Slot::Artifact2 => Self::Artifact,
            Slot::Artifact3 => Self::Artifact,
            Slot::Utility1 => Self::Utility,
            Slot::Utility2 => Self::Utility,
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

#[derive(Debug, Copy, Clone, PartialEq, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
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
            ItemSource::Resource(r) => {
                write!(f, "Resource ({})", r.name())
            }
            ItemSource::Monster(m) => write!(f, "Monster ({})", m.name()),
            ItemSource::Npc(npc_schema) => write!(f, "NPC ({})", npc_schema.name()),
            ItemSource::Craft => write!(f, "Craft"),
            ItemSource::TaskReward => write!(f, "Task Reward"),
            ItemSource::Task => write!(f, "Task"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
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
            LevelConditionCode::AlchemyLevel => Skill::Alchemy,
            LevelConditionCode::MiningLevel => Skill::Mining,
            LevelConditionCode::WoodcuttingLevel => Skill::Woodcutting,
            LevelConditionCode::FishingLevel => Skill::Fishing,
            LevelConditionCode::Level => Skill::Combat,
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
    //TODO: rewrite test
    // use itertools::Itertools;
    // use crate::{items::ItemSchemaExt, };

    // #[test]
    // fn item_damage_against() {
    //     assert_eq!(
    //         ITEMS
    //             .get("skull_staff")
    //             .unwrap()
    //             .attack_damage_against(&MONSTERS.get("ogre").unwrap()),
    //         48.0
    //     );
    //     assert_eq!(
    //         ITEMS
    //             .get("dreadful_staff")
    //             .unwrap()
    //             .attack_damage_against(&MONSTERS.get("vampire").unwrap()),
    //         57.5
    //     );
    // }
    //
    // #[test]
    // fn damage_increase() {
    //     assert_eq!(
    //         ITEMS
    //             .get("steel_boots")
    //             .unwrap()
    //             .damage_increase(super::DamageType::Air),
    //         0
    //     )
    // }
    //
    // #[test]
    // fn damage_increase_against() {
    //     assert_eq!(
    //         ITEMS
    //             .get("steel_armor")
    //             .unwrap()
    //             .damage_increase_against_with(
    //                 &MONSTERS.get("chicken").unwrap(),
    //                 &ITEMS.get("steel_battleaxe").unwrap()
    //             ),
    //         6.0
    //     );
    //
    //     assert_eq!(
    //         ITEMS
    //             .get("steel_boots")
    //             .unwrap()
    //             .damage_increase_against_with(
    //                 &MONSTERS.get("ogre").unwrap(),
    //                 &ITEMS.get("skull_staff").unwrap()
    //             ),
    //         0.0
    //     );
    // }
    //
    // #[test]
    // fn damage_reduction_against() {
    //     assert_eq!(
    //         ITEMS
    //             .get("steel_armor")
    //             .unwrap()
    //             .damage_reduction_against(&MONSTERS.get("ogre").unwrap()),
    //         4.0
    //     );
    // }
    //
    // //#[test]
    // //fn gift_source() {
    // //    assert_eq!(
    // //        ITEMS.sources_of("christmas_star").first(),
    // //        Some(&ItemSource::Gift)
    // //    );
    // //    assert_eq!(
    // //        ITEMS.best_source_of("gift"),
    // //        Some(&ItemSource::Monster(MONSTERS.get("gingerbread").unwrap())).cloned()
    // //    );
    // //}
    //
    // #[test]
    // fn best_consumable_foods() {
    //     assert_eq!(
    //         ITEMS
    //             .best_consumable_foods(29)
    //             .iter()
    //             .max_by_key(|i| i.heal())
    //             .unwrap()
    //             .code,
    //         "cooked_trout"
    //     );
    // }
    //
    // #[test]
    // fn drop_rate() {
    //     assert_eq!(ITEMS.drop_rate("milk_bucket"), 12);
    // }
    //
    // #[test]
    // fn require_task_reward() {
    //     assert!(ITEMS.require_task_reward("greater_dreadful_staff"));
    // }
    //
    // #[test]
    // fn mats_methods() {
    //     assert!(!ITEMS.mats_of("greater_dreadful_staff").is_empty());
    //     assert!(!ITEMS.base_mats_of("greater_dreadful_staff").is_empty());
    // }
}
