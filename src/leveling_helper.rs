use artifactsmmo_sdk::{
    char::{HasCharacterData, Skill},
    items::ItemSchemaExt,
    models::{ItemSchema, MonsterSchema, ResourceSchema},
    resources::RESOURCES,
    ITEMS, MAPS, MONSTERS,
};
use itertools::Itertools;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::sync::{Arc, LazyLock};

use crate::{account::ACCOUNT, bank::BANK, character::Character};

pub static LEVELING_HELPER: LazyLock<LevelingHelper> = LazyLock::new(LevelingHelper::new);

pub struct LevelingHelper {}

impl LevelingHelper {
    fn new() -> Self {
        Self {}
    }

    /// Takes a `level` and a `skill` and returns the items providing experince
    /// when crafted.
    pub fn crafts_providing_exp(
        &self,
        level: i32,
        skill: Skill,
    ) -> impl Iterator<Item = Arc<ItemSchema>> {
        let min = if level > 11 { level - 10 } else { 1 };
        ITEMS
            .all()
            .into_iter()
            .filter(move |i| i.level >= min && i.level <= level)
            .filter(move |i| i.skill_to_craft().is_some_and(|s| s == skill))
    }

    /// Takes a `level` and a `skill` and returns the items of the lowest level
    /// providing experience when crafted.
    pub fn lowest_crafts_providing_exp(&self, level: i32, skill: Skill) -> Vec<Arc<ItemSchema>> {
        self.crafts_providing_exp(level, skill)
            .min_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
    }

    /// Takes a `level` and a `skill` and returns the items of the highest level
    /// providing experience when crafted.
    pub fn highest_crafts_providing_exp(&self, level: i32, skill: Skill) -> Vec<Arc<ItemSchema>> {
        self.crafts_providing_exp(level, skill)
            .max_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
    }

    /// Returns the best items to level the given `skill` at the given `level.
    pub fn best_crafts_hardcoded(&self, level: i32, skill: Skill) -> Vec<Arc<ItemSchema>> {
        match skill {
            Skill::Gearcrafting => {
                if level >= 20 {
                    return self.best_crafts(level, skill);
                } else if level >= 10 {
                    vec![ITEMS.get("iron_helm")]
                //} else if level >= 5 {
                //    vec![self.get("copper_legs_armor")]
                } else {
                    vec![ITEMS.get("wooden_shield")]
                }
            }
            Skill::Weaponcrafting => {
                return self.best_crafts(level, skill);
            }
            Skill::Jewelrycrafting => {
                if level >= 30 {
                    vec![ITEMS.get("gold_ring")]
                } else if level >= 20 {
                    vec![ITEMS.get("steel_ring")]
                } else if level >= 15 {
                    vec![ITEMS.get("life_ring")]
                } else if level >= 10 {
                    vec![ITEMS.get("iron_ring")]
                } else {
                    vec![ITEMS.get("copper_ring")]
                }
            }
            Skill::Cooking => {
                if level >= 30 {
                    vec![ITEMS.get("cooked_bass")]
                } else if level >= 20 {
                    vec![ITEMS.get("cooked_trout")]
                } else if level >= 10 {
                    vec![ITEMS.get("cooked_shrimp")]
                } else {
                    vec![ITEMS.get("cooked_gudgeon")]
                }
            }
            Skill::Mining | Skill::Woodcutting | Skill::Alchemy => {
                return self.best_crafts(level, skill)
            }
            Skill::Fishing => vec![None],
            Skill::Combat => vec![None],
        }
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn best_crafts(&self, level: i32, skill: Skill) -> Vec<Arc<ItemSchema>> {
        self.crafts_providing_exp(level, skill)
            .filter(|i| {
                ![
                    "wooden_staff",
                    "life_amulet",
                    "feather_coat",
                    "ruby",
                    "diamond",
                    "emerald",
                    "sapphire",
                    "topaz",
                ]
                .contains(&i.code.as_str())
                    && !ITEMS.require_task_reward(&i.code)
                    && !i.is_crafted_from_task()
                    && !i.is_crafted_with("obsidian")
                    && !i.is_crafted_with("diamond")
                    && i.mats()
                        .iter()
                        .all(|m| ITEMS.get(&m.code).unwrap().level <= level)
            })
            .max_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
    }

    pub fn best_craft(
        &self,
        level: i32,
        skill: Skill,
        char: &Character,
    ) -> Option<Arc<ItemSchema>> {
        self.best_crafts_hardcoded(level, skill)
            .into_iter()
            .filter_map(|i| {
                let mats_with_ttg = BANK
                    .missing_mats_for(
                        &i.code,
                        char.max_craftable_items(&i.code),
                        Some(&char.name()),
                    )
                    .into_iter()
                    .par_bridge()
                    .map(|m| (m.clone(), ACCOUNT.time_to_get(&m.code)))
                    .collect::<Vec<_>>();
                if mats_with_ttg.iter().all(|(_, ttg)| ttg.is_some()) {
                    Some((
                        i,
                        mats_with_ttg
                            .iter()
                            .filter_map(|(m, ttg)| ttg.as_ref().map(|ttg| (ttg * m.quantity)))
                            .sum::<i32>(),
                    ))
                } else {
                    None
                }
            })
            .min_by_key(|(_, ttg)| *ttg)
            .map(|(i, _)| i)
    }

    pub fn best_resource(&self, level: i32, skill: Skill) -> Option<Arc<ResourceSchema>> {
        RESOURCES
            .all()
            .into_iter()
            .filter(|r| {
                Skill::from(r.skill) == skill
                    && r.level <= level
                    && level - r.level <= 10
                    && !MAPS.with_content_code(&r.code).is_empty()
            })
            .max_by_key(|r| r.level)
    }

    pub fn best_monster(&self, char: &Character) -> Option<Arc<MonsterSchema>> {
        MONSTERS
            .all()
            .into_iter()
            .filter(|m| char.level() >= m.level && m.code != "imp" && m.code != "death_knight")
            .max_by_key(|m| if char.can_kill(m).is_ok() { m.level } else { 0 })
    }
}
