use super::{
    character::Character,
    fight_simulator::FightSimulator,
    gear::Gear,
    items::{Items, Type},
    skill::Skill,
    ItemSchemaExt,
};
use artifactsmmo_openapi::models::{FightResult, ItemSchema, MonsterSchema};
use itertools::{Itertools, PeekingNext};
use ordered_float::OrderedFloat;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{sync::Arc, vec};

pub struct GearFinder {
    items: Arc<Items>,
    fight_simulator: FightSimulator,
}

impl GearFinder {
    pub fn new(items: &Arc<Items>) -> Self {
        Self {
            items: items.clone(),
            fight_simulator: FightSimulator::new(),
        }
    }

    pub fn best_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Gear<'_> {
        if let Some(gear) = self
            .bests_against(char, monster, filter)
            .into_iter()
            .filter(|g| {
                self.fight_simulator
                    .simulate(char.level(), 0, g, monster)
                    .result
                    == FightResult::Win
            })
            .min_set_by_key(|g| {
                self.fight_simulator
                    .simulate(char.level(), 0, g, monster)
                    .turns
            })
            .into_iter()
            .min_by_key(|g| {
                self.fight_simulator
                    .simulate(char.level(), 0, g, monster)
                    .hp_lost
            })
        {
            gear
        } else {
            Default::default()
        }
    }

    pub fn bests_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Vec<Gear<'_>> {
        self.items
            .equipable_at_level(char.level(), Type::Weapon)
            .iter()
            .filter(|i| Self::is_eligible(i, filter, char))
            .flat_map(|w| self.bests_against_with_weapon(char, monster, filter, w))
            .collect_vec()
    }

    fn bests_against_with_weapon<'a>(
        &'a self,
        char: &Character,
        monster: &MonsterSchema,
        filter: Filter,
        weapon: &'a ItemSchema,
    ) -> Vec<Gear> {
        let helmets = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Helmet,
            filter,
            vec![],
        );
        let shields = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Shield,
            filter,
            vec![],
        );
        let body_armor = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::BodyArmor,
            filter,
            vec![],
        );
        let leg_armor = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::LegArmor,
            filter,
            vec![],
        );
        let boots = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Boots,
            filter,
            vec![],
        );
        let rings =
            self.best_armors_against_with_weapon(char, monster, weapon, Type::Ring, filter, vec![]);
        let ring2_black_list = rings
            .iter()
            .filter(|i| {
                if filter == Filter::Available {
                    char.has_available(&i.code) <= 1
                } else {
                    true
                }
            })
            .map(|i| i.code.as_str())
            .collect_vec();
        let rings2 = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Ring,
            filter,
            ring2_black_list,
        );
        let amulets = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Amulet,
            filter,
            vec![],
        );
        // TODO: handle artifacts and consumables
        //let artifacts = self.best_armors_against_with_weapon(char, monster, weapon, Type::Artifact);
        //let consumables =
        //    self.best_armors_against_with_weapon(char, monster, weapon, Type::Consumable);
        let mut items = vec![];
        if !helmets.is_empty() {
            items.push(helmets);
        }
        if !shields.is_empty() {
            items.push(shields);
        }
        if !body_armor.is_empty() {
            items.push(body_armor);
        }
        if !leg_armor.is_empty() {
            items.push(leg_armor);
        }
        if !boots.is_empty() {
            items.push(boots);
        }
        if !rings.is_empty() {
            items.push(rings);
        }
        if !rings2.is_empty() {
            items.push(rings2);
        }
        if !amulets.is_empty() {
            items.push(amulets);
        }
        items
            .into_iter()
            .multi_cartesian_product()
            .map(|items| {
                let mut iter = items.into_iter().peekable();
                Gear {
                    weapon: Some(weapon),
                    helmet: iter.peeking_next(|i| i.is_of_type(Type::Helmet)),
                    shield: iter.peeking_next(|i| i.is_of_type(Type::Shield)),
                    body_armor: iter.peeking_next(|i| i.is_of_type(Type::BodyArmor)),
                    leg_armor: iter.peeking_next(|i| i.is_of_type(Type::LegArmor)),
                    boots: iter.peeking_next(|i| i.is_of_type(Type::Boots)),
                    ring1: iter.peeking_next(|i| i.is_of_type(Type::Ring)),
                    ring2: iter.peeking_next(|i| i.is_of_type(Type::Ring)),
                    amulet: iter.peeking_next(|i| i.is_of_type(Type::Amulet)),
                    artifact1: iter.peeking_next(|i| i.is_of_type(Type::Artifact)),
                    artifact2: iter.peeking_next(|i| i.is_of_type(Type::Artifact)),
                    artifact3: iter.peeking_next(|i| i.is_of_type(Type::Artifact)),
                    utility1: iter.peeking_next(|i| i.is_of_type(Type::Utility)),
                    utility2: iter.peeking_next(|i| i.is_of_type(Type::Utility)),
                }
            })
            .collect::<Vec<_>>()
    }

    fn best_armors_against_with_weapon(
        &self,
        char: &Character,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        r#type: Type,
        filter: Filter,
        black_list: Vec<&str>,
    ) -> Vec<&ItemSchema> {
        let mut upgrades: Vec<&ItemSchema> = vec![];
        let equipables = self
            .items
            .equipable_at_level(char.level(), r#type)
            .into_iter()
            .filter(|i| Self::is_eligible(i, filter, char))
            .filter(|i| !black_list.contains(&i.code.as_str()))
            .collect_vec();
        let damage_increases = equipables
            .iter()
            .cloned()
            .filter(|i| i.damage_increase_against_with(monster, weapon) > 0.0)
            .collect_vec();
        let best_for_damage = damage_increases
            .iter()
            .cloned()
            .max_by_key(|i| OrderedFloat(i.damage_increase_against_with(monster, weapon)));
        let damage_reductions = equipables
            .iter()
            .cloned()
            .filter(|i| i.damage_reduction_against(monster) > 0.0)
            .collect_vec();
        let best_reduction = damage_reductions
            .iter()
            .cloned()
            .max_by_key(|i| OrderedFloat(i.damage_reduction_against(monster)));
        if let Some(best_for_damage) = best_for_damage {
            upgrades.push(best_for_damage);
        }
        if let Some(best_reduction) = best_reduction {
            upgrades.push(best_reduction);
        }
        //let best_for_health = {
        //    if best_for_damage.is_some() {
        //        damage_increases.into_iter().max_by_key(|i| i.health())
        //    } else {
        //        equipables.iter().cloned().max_by_key(|i| i.health())
        //    }
        //};
        //if let Some(best_for_health) = best_for_health {
        //    upgrades.push(best_for_health);
        //}
        upgrades.sort_by_key(|i| &i.code);
        upgrades.dedup_by_key(|i| &i.code);
        upgrades
    }

    pub fn best_tool(&self, char: &Character, skill: Skill, filter: Filter) -> Option<&ItemSchema> {
        self.items
            .equipable_at_level(char.level(), Type::Weapon)
            .into_iter()
            .filter(|i| match filter {
                Filter::All => true,
                Filter::Available => char.has_available(&i.code) > 0,
                Filter::Craftable => char.account.can_craft(&i.code),
                Filter::Farmable => todo!(),
            })
            .min_by_key(|i| i.skill_cooldown_reduction(skill))
    }

    fn is_eligible(i: &ItemSchema, filter: Filter, char: &Character) -> bool {
        match filter {
            Filter::All => {
                i.code != "lizard_skin_armor"
                    && i.code != "lizard_skin_legs_armor"
                    && i.code != "piggy_armor"
                    && i.code != "piggy_pants"
                    && i.code != "serpent_skin_armor"
                    && i.code != "serpent_skin_legs_armor"
                    && i.code != "stormforged_armor"
                    && i.code != "stormforged_pants"
            }
            Filter::Available => char.has_available(&i.code) > 0,
            Filter::Craftable => {
                (i.craft_schema().is_none() || char.account.can_craft(&i.code))
                    && !i.is_crafted_with("jasper_crystal")
                    && i.code != "lizard_skin_armor"
                    && i.code != "lizard_skin_legs_armor"
                    && i.code != "piggy_armor"
                    && i.code != "piggy_pants"
                    && i.code != "serpent_skin_armor"
                    && i.code != "serpent_skin_legs_armor"
                    && i.code != "stormforged_armor"
                    && i.code != "stormforged_pants"
            }
            Filter::Farmable => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Filter {
    All,
    Available,
    Craftable,
    Farmable,
}
