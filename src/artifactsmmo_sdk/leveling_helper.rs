use super::{
    character::Character, items::Items, maps::Maps, monsters::Monsters, resources::Resources,
    skill::Skill, ItemSchemaExt,
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
}

impl LevelingHelper {
    pub fn new(
        items: &Arc<Items>,
        resources: &Arc<Resources>,
        monsters: &Arc<Monsters>,
        maps: &Arc<Maps>,
    ) -> Self {
        Self {
            items: items.clone(),
            resources: resources.clone(),
            monsters: monsters.clone(),
            maps: maps.clone(),
        }
    }

    /// Takes a `level` and a `skill` and returns the items providing experince
    /// when crafted.
    pub fn crafts_providing_exp(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.items
            .data
            .values()
            .filter(|i| i.level >= min && i.level <= level)
            .filter(|i| i.skill_to_craft().is_some_and(|s| s == skill))
            .collect_vec()
    }

    /// Takes a `level` and a `skill` and returns the items of the lowest level
    /// providing experience when crafted.
    pub fn lowest_crafts_providing_exp(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        self.crafts_providing_exp(level, skill)
            .iter()
            .min_set_by_key(|i| i.level)
            .into_iter()
            .cloned()
            .collect_vec()
    }

    /// Takes a `level` and a `skill` and returns the items of the highest level
    /// providing experience when crafted.
    pub fn highest_crafts_providing_exp(&self, level: i32, skill: Skill) -> Vec<&ItemSchema> {
        self.crafts_providing_exp(level, skill)
            .iter()
            .max_set_by_key(|i| i.level)
            .into_iter()
            .cloned()
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
            .into_iter()
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
                    && !i.is_crafted_with("jasper_crystal")
                    && !i.is_crafted_with("magical_cure")
            })
            .filter(|i| {
                i.mats()
                    .iter()
                    .all(|m| self.items.get(&m.code).unwrap().level <= level)
            })
            .max_set_by_key(|i| i.level)
            .into_iter()
            .collect_vec()
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
