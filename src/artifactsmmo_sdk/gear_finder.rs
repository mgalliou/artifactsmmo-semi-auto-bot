use super::{
    character::Character,
    fight_simulator::FightSimulator,
    gear::Gear,
    items::{
        Items, Type, ASTRALYTE_CRYSTAL, DIAMOND, ENCHANTED_FABRIC, JASPER_CRYSTAL, MAGICAL_CURE,
    },
    skill::Skill,
    ItemSchemaExt,
};
use artifactsmmo_openapi::models::{FightResult, ItemSchema, MonsterSchema};
use itertools::{Itertools, PeekingNext};
use ordered_float::OrderedFloat;
use std::sync::Arc;
use strum_macros::EnumString;

#[derive(Default)]
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
        self.bests_against(char, monster, filter)
            .into_iter()
            .map(|g| {
                (
                    g,
                    self.fight_simulator.simulate(char.level(), 0, &g, monster),
                )
            })
            .filter(|(_g, f)| f.result == FightResult::Win)
            .min_by_key(|(_g, f)| f.cd + f.hp_lost / 5 + if f.hp_lost % 5 > 0 { 1 } else { 0 })
            .map(|(g, _f)| g)
            .unwrap_or_default()
    }

    pub fn bests_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Vec<Gear<'_>> {
        self.best_weapons_against(char, monster, filter)
            .iter()
            .flat_map(|w| self.bests_against_with_weapon(char, monster, filter, w))
            .collect_vec()
    }

    pub fn best_weapons_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Vec<&'a ItemSchema> {
        let equipables = self
            .items
            .equipable_at_level(char.level(), Type::Weapon)
            .into_iter()
            .filter(|i| Self::is_eligible(i, filter, char))
            .collect_vec();
        let best = equipables
            .iter()
            .max_by_key(|i| OrderedFloat(i.attack_damage_against(monster)))
            .cloned();
        equipables
            .into_iter()
            .filter(|i| {
                if let Some(best) = best {
                    i.attack_damage_against(monster) > best.attack_damage_against(monster) * 0.75
                } else {
                    false
                }
            })
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
        let amulets = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Amulet,
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
                    false
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
        let artifacts = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Artifact,
            filter,
            vec![],
        );
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
        if !amulets.is_empty() {
            items.push(amulets);
        }
        if !rings.is_empty() {
            items.push(rings);
        }
        if !rings2.is_empty() {
            items.push(rings2);
        }
        if !artifacts.is_empty() {
            items.push(artifacts.clone());
        }
        if !artifacts.is_empty() {
            items.push(artifacts.clone());
        }
        if !artifacts.is_empty() {
            items.push(artifacts);
        }
        // TODO: handle artifacts and consumables
        //let consumables =
        //    self.best_armors_against_with_weapon(char, monster, weapon, Type::Consumable);
        items
            .into_iter()
            .multi_cartesian_product()
            .filter_map(|items| {
                let mut iter = items.into_iter().peekable();
                let utility1 = iter.peeking_next(|i| i.is_of_type(Type::Utility));
                let utility2 = iter.peeking_next(|i| i.is_of_type(Type::Utility));
                let artifact2 = iter.peeking_next(|i| i.is_of_type(Type::Artifact));
                let artifact3 = iter.peeking_next(|i| i.is_of_type(Type::Artifact));
                if utility1.is_some_and(|u1| utility2.is_some_and(|u2| u1.code == u2.code)) {
                    None
                } else {
                    Some(Gear {
                        weapon: Some(weapon),
                        helmet: iter.peeking_next(|i| i.is_of_type(Type::Helmet)),
                        shield: iter.peeking_next(|i| i.is_of_type(Type::Shield)),
                        body_armor: iter.peeking_next(|i| i.is_of_type(Type::BodyArmor)),
                        leg_armor: iter.peeking_next(|i| i.is_of_type(Type::LegArmor)),
                        boots: iter.peeking_next(|i| i.is_of_type(Type::Boots)),
                        amulet: iter.peeking_next(|i| i.is_of_type(Type::Amulet)),
                        ring1: iter.peeking_next(|i| i.is_of_type(Type::Ring)),
                        ring2: iter.peeking_next(|i| i.is_of_type(Type::Ring)),
                        utility1,
                        utility2,
                        artifact1: iter.peeking_next(|i| i.is_of_type(Type::Artifact)),
                        artifact2,
                        artifact3,
                    })
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
            Filter::All => true,
            Filter::Available => char.has_available(&i.code) > 0,
            Filter::Craftable => {
                (i.craft_schema().is_none() || char.account.can_craft(&i.code))
                    && !i.is_crafted_with(JASPER_CRYSTAL)
                    && !i.is_crafted_with(MAGICAL_CURE)
                    && !i.is_crafted_with(ENCHANTED_FABRIC)
                    && !i.is_crafted_with(ASTRALYTE_CRYSTAL)
                    && !i.is_crafted_with(DIAMOND)
                    && ![
                        "lizard_skin_armor",
                        "lizard_skin_armor",
                        "lizard_skin_legs_armor",
                        "piggy_armor",
                        "piggy_pants",
                        "serpent_skin_armor",
                        "serpent_skin_legs_armor",
                        "stormforged_armor",
                        "stormforged_pants",
                        "lich_crown",
                        "life_crystal",
                    ]
                    .contains(&i.code.as_str())
            }
            Filter::Farmable => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Filter {
    All,
    Available,
    Craftable,
    Farmable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifactsmmo_sdk::{
        game_config::GameConfig, monsters::Monsters, resources::Resources, tasks::Tasks,
    };

    #[test]
    fn best_weapons_against() {
        let config: Arc<GameConfig> = Arc::new(GameConfig::from_file());
        let events = Default::default();
        let resources = Arc::new(Resources::new(&config, &events));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));
        let gear_finder = GearFinder::new(&items);
        let char = Character::default();
        char.data.write().unwrap().level = 30;

        let weapons = gear_finder.best_weapons_against(
            &char,
            monsters.get("vampire").unwrap(),
            Filter::Craftable,
        );
        assert_eq!(weapons, vec![items.get("death_knight_sword").unwrap()]);
    }
}
