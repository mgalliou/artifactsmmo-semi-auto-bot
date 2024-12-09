use super::{
    character::Character,
    fight_simulator::FightSimulator,
    gear::Gear,
    items::{Items, Type},
    skill::Skill,
    ItemSchemaExt,
};
use artifactsmmo_openapi::models::{FightResult, ItemSchema, MonsterSchema};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::{collections::HashSet, sync::Arc};

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

    pub fn best_winning_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Gear<'a> {
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

    pub fn best_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Gear<'a> {
        self.bests_against(char, monster, filter)
            .into_iter()
            .map(|g| {
                (
                    g,
                    self.fight_simulator.simulate(char.level(), 0, &g, monster),
                )
            })
            .min_by_key(|(_g, f)| f.monster_hp)
            .map(|(g, _f)| g)
            .unwrap_or_default()
    }

    pub fn bests_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Vec<Gear<'a>> {
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
            .filter(|i| self.is_eligible(i, filter, char))
            .collect_vec();
        let best = equipables
            .iter()
            .max_by_key(|i| OrderedFloat(i.attack_damage_against(monster)))
            .cloned();
        equipables
            .into_iter()
            .filter(|i| {
                if let Some(best) = best {
                    i.attack_damage_against(monster) >= best.attack_damage_against(monster) * 0.90
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
    ) -> Vec<Gear<'a>> {
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
            .flatten()
            .filter(|i| {
                if filter.available {
                    char.has_available(i) <= 1
                } else {
                    false
                }
            })
            .cloned()
            .collect_vec();
        let rings2 = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Ring,
            filter,
            ring2_black_list,
        );
        let mut ring_sets = [rings, rings2]
            .iter()
            .multi_cartesian_product()
            .map(|rings| [*rings[0], *rings[1]])
            .sorted()
            .map(|rings| ItemWrapper::Rings(RingSet::new(rings)))
            .collect_vec();
        ring_sets.dedup();
        let mut artifacts = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Artifact,
            filter,
            vec![],
        );
        artifacts.push(None);
        let artifact_sets = [artifacts.clone(), artifacts.clone(), artifacts]
            .iter()
            .multi_cartesian_product()
            .map(|artifacts| {
                ItemWrapper::Artifacts({
                    let mut set = HashSet::new();
                    set.insert(*artifacts[0]);
                    set.insert(*artifacts[1]);
                    set.insert(*artifacts[2]);
                    set
                })
            })
            .collect_vec();
        let mut items = vec![];
        if !helmets.is_empty() {
            items.push(helmets.iter().map(|i| ItemWrapper::Armor(*i)).collect_vec());
        }
        if !shields.is_empty() {
            items.push(shields.iter().map(|i| ItemWrapper::Armor(*i)).collect_vec());
        }
        if !body_armor.is_empty() {
            items.push(
                body_armor
                    .iter()
                    .map(|i| ItemWrapper::Armor(*i))
                    .collect_vec(),
            );
        }
        if !leg_armor.is_empty() {
            items.push(
                leg_armor
                    .iter()
                    .map(|i| ItemWrapper::Armor(*i))
                    .collect_vec(),
            );
        }
        if !boots.is_empty() {
            items.push(boots.iter().map(|i| ItemWrapper::Armor(*i)).collect_vec());
        }
        if !amulets.is_empty() {
            items.push(amulets.iter().map(|i| ItemWrapper::Armor(*i)).collect_vec());
        }
        if !ring_sets.is_empty() {
            items.push(ring_sets);
        }
        if !artifact_sets.is_empty() {
            items.push(artifact_sets);
        }
        // TODO: handle artifacts and consumables
        //let consumables =
        //    self.best_armors_against_with_weapon(char, monster, weapon, Type::Consumable);
        items
            .iter()
            .multi_cartesian_product()
            .filter_map(|items| {
                Gear::new(
                    Some(weapon),
                    self.item_from_wrappers(&items, Type::Helmet, 0),
                    self.item_from_wrappers(&items, Type::Shield, 0),
                    self.item_from_wrappers(&items, Type::BodyArmor, 0),
                    self.item_from_wrappers(&items, Type::LegArmor, 0),
                    self.item_from_wrappers(&items, Type::Boots, 0),
                    self.item_from_wrappers(&items, Type::Amulet, 0),
                    self.item_from_wrappers(&items, Type::Ring, 0),
                    self.item_from_wrappers(&items, Type::Ring, 1),
                    self.item_from_wrappers(&items, Type::Utility, 0),
                    self.item_from_wrappers(&items, Type::Utility, 1),
                    self.item_from_wrappers(&items, Type::Artifact, 0),
                    self.item_from_wrappers(&items, Type::Artifact, 1),
                    self.item_from_wrappers(&items, Type::Artifact, 2),
                )
            })
            .collect::<Vec<_>>()
    }

    fn item_from_wrappers(
        &self,
        wrapper: &Vec<&ItemWrapper>,
        r#type: Type,
        index: usize,
    ) -> Option<&ItemSchema> {
        wrapper.iter().find_map(|w| match w {
            ItemWrapper::Armor(Some(armor)) => {
                self.items
                    .get(armor)
                    .and_then(|i| if i.is_of_type(r#type) { Some(i) } else { None })
            }
            ItemWrapper::Armor(None) => None,
            ItemWrapper::Rings(ring_set) => {
                if let Some(Some(ring)) = ring_set.rings.get(index) {
                    self.items.get(ring).and_then(
                        |i| {
                            if i.is_of_type(r#type) {
                                Some(i)
                            } else {
                                None
                            }
                        },
                    )
                } else {
                    None
                }
            }
            ItemWrapper::Artifacts(set) => {
                if let Some(Some(artifact)) = set.iter().collect_vec().get(index) {
                    self.items.get(artifact).and_then(|i| {
                        if i.is_of_type(r#type) {
                            Some(i)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            }
            ItemWrapper::Utility(set) => {
                if let Some(Some(utility)) = set.iter().collect_vec().get(index) {
                    self.items.get(utility).and_then(|i| {
                        if i.is_of_type(r#type) {
                            Some(i)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            }
        })
    }

    fn best_armors_against_with_weapon(
        &self,
        char: &Character,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        r#type: Type,
        filter: Filter,
        black_list: Vec<&str>,
    ) -> Vec<Option<&str>> {
        let mut upgrades: Vec<&ItemSchema> = vec![];
        let equipables = self
            .items
            .equipable_at_level(char.level(), r#type)
            .into_iter()
            .filter(|i| !black_list.contains(&i.code.as_str()) && self.is_eligible(i, filter, char))
            .collect_vec();
        let best_for_damage = equipables
            .iter()
            .filter(|i| i.damage_increase_against_with(monster, weapon) > 0.0)
            .max_by_key(|i| OrderedFloat(i.damage_increase_against_with(monster, weapon)));
        let best_reduction = equipables
            .iter()
            .filter(|i| i.damage_reduction_against(monster) > 0.0)
            .max_by_key(|i| OrderedFloat(i.damage_reduction_against(monster)));
        let best_health_increase = equipables
            .iter()
            .filter(|i| i.health() > 0)
            .max_by_key(|i| i.health());
        if let Some(best_for_damage) = best_for_damage {
            upgrades.push(best_for_damage);
        }
        if let Some(best_reduction) = best_reduction {
            upgrades.push(best_reduction);
        }
        if let Some(best_health_increase) = best_health_increase {
            upgrades.push(best_health_increase);
        }
        upgrades.sort_by_key(|i| &i.code);
        upgrades.dedup_by_key(|i| &i.code);
        upgrades
            .into_iter()
            .map(|i| Some(i.code.as_str()))
            .collect_vec()
    }

    pub fn best_tool(&self, char: &Character, skill: Skill, filter: Filter) -> Option<&ItemSchema> {
        self.items
            .equipable_at_level(char.level(), Type::Weapon)
            .into_iter()
            .filter(|i| self.is_eligible(i, filter, char) && i.skill_cooldown_reduction(skill) < 0)
            .min_by_key(|i| i.skill_cooldown_reduction(skill))
    }

    fn is_eligible(&self, i: &ItemSchema, filter: Filter, char: &Character) -> bool {
        if filter.available && char.has_available(&i.code) > 0 {
            return true;
        }
        if filter.available && char.has_available(&i.code) <= 0 {
            return false;
        }
        if i.code == "sanguine_edge_of_rosen" {
            return false;
        }
        if filter.can_craft && i.craft_schema().is_some() && !char.account.can_craft(&i.code) {
            return false;
        }
        if !filter.from_task && i.is_crafted_from_task() {
            return false;
        }
        if !filter.from_monster
            && self
                .items
                .best_source_of(&i.code)
                .is_some_and(|s| s.is_monster())
        {
            return false;
        }
        if !filter.from_gift
            && self
                .items
                .best_source_of(&i.code)
                .is_some_and(|s| s.is_gift())
        {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Filter {
    pub available: bool,
    pub from_monster: bool,
    pub from_task: bool,
    pub can_craft: bool,
    pub from_gift: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            available: false,
            can_craft: false,
            from_task: true,
            from_monster: true,
            from_gift: false,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ItemWrapper<'a> {
    Armor(Option<&'a str>),
    Rings(RingSet<'a>),
    Artifacts(HashSet<Option<&'a str>>),
    Utility(HashSet<Option<&'a str>>),
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
struct RingSet<'a> {
    rings: [Option<&'a str>; 2],
}

impl<'a> RingSet<'a> {
    fn new(mut rings: [Option<&'a str>; 2]) -> Self {
        rings.sort();
        RingSet { rings }
    }
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
            Default::default(),
        );
        assert_eq!(weapons, vec![items.get("death_knight_sword").unwrap()]);
    }
}
