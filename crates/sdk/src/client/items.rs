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

#[derive(Default, Debug, Clone, CollectionClient)]
pub struct ItemsClient(Arc<ItemsClientInner>);

#[derive(Default, Debug)]
pub struct ItemsClientInner {
    api: ArtifactApi,
    data: RwLock<HashMap<String, Item>>,
    resources: ResourcesClient,
    monsters: MonstersClient,
    tasks_rewards: TasksRewardsClient,
    npcs: NpcsClient,
}

impl ItemsClient {
    pub(crate) fn new(
        api: ArtifactApi,
        resources: ResourcesClient,
        monsters: MonstersClient,
        tasks_rewards: TasksRewardsClient,
        npcs: NpcsClient,
    ) -> Self {
        let items = Self(
            ItemsClientInner {
                api,
                data: RwLock::default(),
                resources,
                monsters,
                tasks_rewards,
                npcs,
            }
            .into(),
        );
        *items.0.data.write().unwrap() = items.load();
        items
    }

    /// Takes an item `code` and return the mats required to craft it.
    pub fn mats_of(&self, code: &str) -> Vec<SimpleItemSchema> {
        self.get(code).iter().flat_map(Item::mats).collect_vec()
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
        self.0
            .resources
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
    pub fn mats_quantity_for(&self, code: &str) -> u32 {
        self.mats_of(code).iter().map(|mat| mat.quantity).sum()
    }

    pub fn recycled_quantity_for(&self, code: &str) -> u32 {
        let mats_quantity_for = self.mats_quantity_for(code);
        mats_quantity_for / 5 + u32::from(!mats_quantity_for.is_multiple_of(5))
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
            .0
            .resources
            .dropping(code)
            .into_iter()
            .map(ItemSource::Resource)
            .collect_vec();
        sources.extend(
            self.0
                .monsters
                .dropping(code)
                .into_iter()
                .map(ItemSource::Monster)
                .collect_vec(),
        );
        if self.get(code).is_some_and(|i| i.is_craftable()) {
            sources.push(ItemSource::Craft);
        }
        if self.0.tasks_rewards.all().iter().any(|r| r.code() == code) {
            sources.push(ItemSource::TaskReward);
        }
        sources.extend(
            self.0
                .npcs
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
                ItemSource::Resource(resource) => self.0.resources.is_event(resource.code()),
                ItemSource::Monster(monster) => self.0.monsters.is_event(monster.code()),
                ItemSource::Npc(npc) => npc.is_merchant(),
                ItemSource::Craft | ItemSource::TaskReward | ItemSource::Task => false,
            })
        })
    }

    pub fn is_buyable(&self, item_code: &str) -> bool {
        self.0
            .npcs
            .items()
            .get(item_code)
            .is_some_and(|i| i.is_buyable())
    }

    pub fn is_salable(&self, item_code: &str) -> bool {
        self.0
            .npcs
            .items()
            .get(item_code)
            .is_some_and(|i| i.is_salable())
    }
}

impl Persist<HashMap<String, Item>> for ItemsClient {
    const PATH: &'static str = ".cache/items.json";

    fn load_from_api(&self) -> HashMap<String, Item> {
        self.0
            .api
            .items
            .get_all()
            .unwrap()
            .into_iter()
            .map(|i| (i.code.clone(), Item::new(i)))
            .collect()
    }

    fn refresh(&self) {
        *self.0.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for ItemsClient {
    type Entity = Item;
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
