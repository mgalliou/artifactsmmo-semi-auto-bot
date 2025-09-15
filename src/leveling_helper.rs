use artifactsmmo_sdk::{
    CanProvideXp, CollectionClient, ItemsClient, Level, MapsClient, MonstersClient,
    ResourcesClient,
    character::HasCharacterData,
    items::{ItemSchemaExt, SubType},
    models::{ItemSchema, MonsterSchema, ResourceSchema},
    skill::Skill,
};
use itertools::Itertools;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::sync::Arc;

use crate::{account::AccountController, bank::BankController, character::CharacterController};

#[derive(Default)]
pub struct LevelingHelper {
    account: Arc<AccountController>,
    bank: Arc<BankController>,
    items: Arc<ItemsClient>,
    monsters: Arc<MonstersClient>,
    resources: Arc<ResourcesClient>,
    maps: Arc<MapsClient>,
}

impl LevelingHelper {
    pub fn new(
        items: Arc<ItemsClient>,
        monsters: Arc<MonstersClient>,
        resources: Arc<ResourcesClient>,
        maps: Arc<MapsClient>,
        account: Arc<AccountController>,
        bank: Arc<BankController>,
    ) -> Self {
        Self {
            items,
            monsters,
            resources,
            maps,
            account,
            bank,
        }
    }

    /// Takes a `level` and a `skill` and returns the items providing experince
    /// when crafted.
    pub fn crafts_providing_exp(
        &self,
        level: u32,
        skill: Skill,
    ) -> impl Iterator<Item = Arc<ItemSchema>> {
        self.items
            .filtered(|i| i.skill_to_craft_is(skill) && i.provides_xp_at(level))
            .into_iter()
    }

    /// Takes a `level` and a `skill` and returns the items of the lowest level
    /// providing experience when crafted.
    pub fn lowest_crafts_providing_exp(&self, level: u32, skill: Skill) -> Vec<Arc<ItemSchema>> {
        self.crafts_providing_exp(level, skill)
            .min_set_by_key(|i| i.level)
    }

    /// Takes a `level` and a `skill` and returns the items of the highest level
    /// providing experience when crafted.
    pub fn highest_crafts_providing_exp(&self, level: u32, skill: Skill) -> Vec<Arc<ItemSchema>> {
        self.crafts_providing_exp(level, skill)
            .max_set_by_key(|i| i.level)
    }

    pub fn best_craft(
        &self,
        level: u32,
        skill: Skill,
        char: &CharacterController,
    ) -> Option<Arc<ItemSchema>> {
        self.best_crafts_hardcoded(level, skill)
            .into_iter()
            .filter_map(|i| {
                let mats = self
                    .items
                    .mats_for(&i.code, char.max_craftable_items(&i.code));
                let mats_with_ttg = self
                    .bank
                    .missing_among(&mats, &char.name())
                    .into_iter()
                    .par_bridge()
                    .map(|m| (m.clone(), self.account.time_to_get(&m.code)))
                    .collect::<Vec<_>>();
                mats_with_ttg
                    .iter()
                    .all(|(_, ttg)| ttg.is_some())
                    .then_some((
                        i,
                        mats_with_ttg
                            .iter()
                            .filter_map(|(m, ttg)| ttg.as_ref().map(|ttg| (ttg * m.quantity)))
                            .sum::<u32>(),
                    ))
            })
            .min_by_key(|(_, ttg)| *ttg)
            .map(|(i, _)| i)
    }

    /// Returns the best items to level the given `skill` at the given `level.
    pub fn best_crafts_hardcoded(&self, level: u32, skill: Skill) -> Vec<Arc<ItemSchema>> {
        match skill {
            Skill::Gearcrafting => {
                if level >= 20 {
                    return self.best_crafts(level, skill);
                } else if level >= 10 {
                    vec![self.items.get("iron_helm")]
                //} else if level >= 5 {
                //    vec![self.get("copper_legs_armor")]
                } else {
                    vec![self.items.get("wooden_shield")]
                }
            }
            Skill::Weaponcrafting => {
                return self.best_crafts(level, skill);
            }
            Skill::Jewelrycrafting => {
                if level >= 30 {
                    vec![self.items.get("gold_ring")]
                } else if level >= 20 {
                    vec![self.items.get("steel_ring")]
                } else if level >= 15 {
                    vec![self.items.get("life_ring")]
                } else if level >= 10 {
                    vec![self.items.get("iron_ring")]
                } else {
                    vec![self.items.get("copper_ring")]
                }
            }
            Skill::Cooking => {
                if level >= 30 {
                    vec![self.items.get("cooked_bass")]
                } else if level >= 20 {
                    vec![self.items.get("cooked_trout")]
                } else if level >= 10 {
                    vec![self.items.get("cooked_shrimp")]
                } else {
                    vec![self.items.get("cooked_gudgeon")]
                }
            }
            Skill::Mining | Skill::Woodcutting | Skill::Alchemy => {
                return self.best_crafts(level, skill);
            }
            Skill::Fishing => vec![None],
            Skill::Combat => vec![None],
        }
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn best_crafts(&self, level: u32, skill: Skill) -> Vec<Arc<ItemSchema>> {
        self.crafts_providing_exp(level, skill)
            .filter(|i| {
                !["wooden_staff", "life_amulet", "feather_coat"].contains(&i.code.as_str())
                    && !i.subtype_is(SubType::PreciousStone)
                    && !self.items.require_task_reward(&i.code)
                    && !i.is_crafted_with("obsidian")
                    && !i.is_crafted_with("diamond")
                    && !i.is_crafted_with("strange_ore")
                    && !i.is_crafted_with("magic_wood")
            })
            .max_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
    }

    pub fn best_resource(&self, level: u32, skill: Skill) -> Option<Arc<ResourceSchema>> {
        self.resources
            .filtered(|r| {
                skill == r.skill.into()
                    && r.provides_xp_at(level)
                    && !self.maps.with_content_code(&r.code).is_empty()
            })
            .into_iter()
            .max_by_key(|r| r.level)
    }

    pub fn best_monster(&self, char: &CharacterController) -> Option<Arc<MonsterSchema>> {
        self.monsters
            .filtered(|m| {
                m.level() <= char.level()
                    && !["imp", "death_knight"].contains(&m.code.as_str())
                    && char.can_kill(m).is_ok()
            })
            .into_iter()
            .max_by_key(|m| m.level)
    }
}
