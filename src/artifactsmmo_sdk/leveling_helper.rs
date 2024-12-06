use super::{
    account::Account, character::Character, items::Items, maps::Maps, monsters::Monsters,
    resources::Resources, skill::Skill, ItemSchemaExt,
};
use artifactsmmo_openapi::models::{ItemSchema, MonsterSchema, ResourceSchema};
use itertools::Itertools;
use std::sync::Arc;

#[derive(Default)]
pub struct LevelingHelper {
    items: Arc<Items>,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
    maps: Arc<Maps>,
    account: Arc<Account>,
}

impl LevelingHelper {
    pub fn new(
        items: &Arc<Items>,
        resources: &Arc<Resources>,
        monsters: &Arc<Monsters>,
        maps: &Arc<Maps>,
        account: &Arc<Account>,
    ) -> Self {
        Self {
            items: items.clone(),
            resources: resources.clone(),
            monsters: monsters.clone(),
            maps: maps.clone(),
            account: account.clone(),
        }
    }

    /// Takes a `level` and a `skill` and returns the items providing experince
    /// when crafted.
    pub fn crafts_providing_exp(
        &self,
        level: i32,
        skill: Skill,
    ) -> impl Iterator<Item = &ItemSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.items
            .data
            .values()
            .filter(move |i| i.level >= min && i.level <= level)
            .filter(move |i| i.skill_to_craft().is_some_and(|s| s == skill))
    }

    /// Takes a `level` and a `skill` and returns the items of the lowest level
    /// providing experience when crafted.
    pub fn lowest_crafts_providing_exp(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        self.crafts_providing_exp(level, skill)
            .min_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
    }

    /// Takes a `level` and a `skill` and returns the items of the highest level
    /// providing experience when crafted.
    pub fn highest_crafts_providing_exp(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        self.crafts_providing_exp(level, skill)
            .max_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
    }

    /// Returns the best items to level the given `skill` at the given `level.
    pub fn best_crafts_hardcoded(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
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
                return self.best_crafts(level, skill)
            }
            Skill::Fishing => vec![None],
            Skill::Combat => vec![None],
        }
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn best_crafts(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
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
                    && !self.items.require_task_reward(&i.code)
                    && !i.is_crafted_from_task()
                    && !i.is_crafted_with("obsidian")
                    && !i.is_crafted_with("diamond")
                    && i.mats()
                        .iter()
                        .all(|m| self.items.get(&m.code).unwrap().level <= level)
            })
            .max_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
    }

    pub fn best_craft(&self, level: i32, skill: Skill, char: &Character) -> Option<&ItemSchema> {
        self.best_crafts_hardcoded(level, skill)
            .into_iter()
            .filter_map(|i| {
                let mats_with_ttg = self
                    .account
                    .bank
                    .missing_mats_for(&i.code, char.max_craftable_items(&i.code), Some(&char.name))
                    .into_iter()
                    .map(|m| (m.clone(), self.account.time_to_get(&m.code)))
                    .collect_vec();
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

    pub fn best_resource(&self, level: i32, skill: Skill) -> Option<&ResourceSchema> {
        self.resources
            .data
            .iter()
            .filter(|r| {
                Skill::from(r.skill) == skill
                    && r.level <= level
                    && level - r.level <= 10
                    && !self.maps.with_content_code(&r.code).is_empty()
            })
            .max_by_key(|r| r.level)
    }

    pub fn best_monster(&self, char: &Character) -> Option<&MonsterSchema> {
        self.monsters
            .data
            .iter()
            .filter(|m| char.level() >= m.level && m.code != "imp" && m.code != "death_knight")
            .max_by_key(|m| if char.can_kill(m).is_ok() { m.level } else { 0 })
    }
}
