use crate::{account::AccountController, character::CharacterController};
use artifactsmmo_sdk::{
    Items, Simulator,
    char::{HasCharacterData, Skill},
    gear::Gear,
    items::{ItemSchemaExt, Type},
    models::{ItemSchema, MonsterSchema},
    simulator::HasEffects,
};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::{collections::HashSet, sync::Arc};

#[derive(Default)]
pub struct GearFinder {
    items: Arc<Items>,
    account: Arc<AccountController>,
}

impl GearFinder {
    pub fn new(items: Arc<Items>, account: Arc<AccountController>) -> Self {
        Self { items, account }
    }

    pub fn best_winning_against<'a>(
        &'a self,
        char: &'a CharacterController,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Gear {
        self.bests_against(char, monster, filter)
            .into_iter()
            .filter_map(|g| {
                let fight = Simulator::average_fight(char.level(), 0, &g, monster, false);
                fight.is_winning().then_some((fight, g))
            })
            .min_set_by_key(|(f, _g)| f.cd + Simulator::time_to_rest(f.hp_lost))
            .into_iter()
            .max_by_key(|(f, _g)| f.hp)
            .map(|(_f, g)| g)
            .unwrap_or_default()
    }

    pub fn best_against<'a>(
        &'a self,
        char: &'a CharacterController,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Gear {
        self.bests_against(char, monster, filter)
            .into_iter()
            .map(|g| {
                (
                    Simulator::average_fight(char.level(), 0, &g, monster, true),
                    g,
                )
            })
            .min_set_by_key(|(f, _g)| f.cd + Simulator::time_to_rest(f.hp_lost))
            .into_iter()
            .max_by_key(|(f, _g)| f.hp)
            .map(|(_f, g)| g)
            .unwrap_or_default()
    }

    pub fn bests_against<'a>(
        &'a self,
        char: &'a CharacterController,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Vec<Gear> {
        self.best_weapons_against(char, monster, filter)
            .iter()
            .flat_map(|w| self.bests_against_with_weapon(char, monster, filter, w))
            .collect_vec()
    }

    pub fn best_weapons_against<'a>(
        &'a self,
        char: &'a CharacterController,
        monster: &'a MonsterSchema,
        filter: Filter,
    ) -> Vec<Arc<ItemSchema>> {
        let equipables = self
            .items
            .all()
            .into_iter()
            .filter(|i| {
                i.r#type().is_weapon()
                    && !i.is_tool()
                    && char.meets_conditions_for(i)
                    && self.is_eligible(i, filter, char)
            })
            .collect_vec();
        let best = equipables
            .iter()
            .max_by_key(|i| OrderedFloat(i.attack_damage_against(monster)))
            .cloned();
        equipables
            .into_iter()
            .filter(|i| {
                if let Some(best) = &best {
                    i.attack_damage_against(monster) >= best.attack_damage_against(monster) * 0.90
                } else {
                    false
                }
            })
            .collect_vec()
    }

    fn bests_against_with_weapon(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        filter: Filter,
        weapon: &Arc<ItemSchema>,
    ) -> Vec<Gear> {
        let armor_types = [
            Type::Helmet,
            Type::Shield,
            Type::BodyArmor,
            Type::LegArmor,
            Type::Boots,
            Type::Amulet,
        ];

        let mut items = armor_types
            .iter()
            .filter_map(|&item_type| {
                let armors = self.best_armors_against_with_weapon(
                    char,
                    monster,
                    weapon,
                    item_type,
                    filter,
                    vec![],
                );
                (!armors.is_empty()).then_some(
                    armors
                        .iter()
                        .map(|i| ItemWrapper::Armor(i.clone()))
                        .collect_vec(),
                )
            })
            .collect_vec();

        let ring_sets = self.gen_rings_sets_against(char, monster, weapon, filter);
        if !ring_sets.is_empty() {
            items.push(ring_sets);
        }
        if filter.utilities {
            let utilities_sets = self.gen_utilities_sets_against(char, monster, weapon, filter);
            if !utilities_sets.is_empty() {
                items.push(utilities_sets);
            }
        }
        let artifact_sets = self.gen_artifacts_sets_against(char, monster, weapon, filter);
        if !artifact_sets.is_empty() {
            items.push(artifact_sets);
        }
        self.gen_all_gears(Some(weapon.clone()), items)
    }

    fn gen_rings_sets_against(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
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
            .map(|rings| [rings[0].clone(), rings[1].clone()])
            .sorted()
            .map(|rings| ItemWrapper::Rings(RingSet::new(rings)))
            .collect_vec();
        ring_sets.dedup();
        ring_sets
    }

    fn gen_utilities_sets_against(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let mut utilities =
            self.best_utilities_against_with_weapon(char, monster, weapon, filter, vec![]);
        utilities.push(None);
        let mut sets = [utilities.clone(), utilities]
            .iter()
            .multi_cartesian_product()
            .map(|utilities| {
                ItemWrapper::Utility({
                    let mut set = HashSet::new();
                    set.insert(utilities[0].clone());
                    set.insert(utilities[1].clone());
                    set
                })
            })
            .collect_vec();
        sets.dedup();
        sets
    }

    fn gen_artifacts_sets_against(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let mut artifacts = self.best_armors_against_with_weapon(
            char,
            monster,
            weapon,
            Type::Artifact,
            filter,
            vec![],
        );
        artifacts.push(None);
        let mut sets = [artifacts.clone(), artifacts.clone(), artifacts]
            .iter()
            .multi_cartesian_product()
            .map(|artifacts| {
                [
                    artifacts[0].clone(),
                    artifacts[1].clone(),
                    artifacts[2].clone(),
                ]
            })
            .sorted()
            .filter_map(|artifacts| ArtifactSet::new(artifacts).map(ItemWrapper::Artifacts))
            .collect_vec();
        sets.dedup();
        sets
    }

    fn best_armors_against_with_weapon(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        r#type: Type,
        filter: Filter,
        black_list: Vec<String>,
    ) -> Vec<Option<String>> {
        let mut bests: Vec<Arc<ItemSchema>> = vec![];
        let equipables = self
            .items
            .all()
            .into_iter()
            .filter(|i| {
                !black_list.contains(&i.code)
                    && i.r#type() == r#type
                    && char.meets_conditions_for(i)
                    && self.is_eligible(i, filter, char)
            })
            .collect_vec();
        let best_for_damage = equipables
            .iter()
            .filter(|i| i.damage_increase_against_with(monster, weapon) > 0.0)
            .max_by_key(|i| OrderedFloat(i.damage_increase_against_with(monster, weapon)));
        let bests_for_damage = equipables
            .iter()
            .filter(|i| {
                // TODO: find a better way to handle negative damage reduction on damage increases
                // (snowman_hat)
                if let Some(best) = best_for_damage {
                    i.damage_increase_against_with(monster, weapon)
                        >= best.damage_increase_against_with(monster, weapon) * 0.75
                } else {
                    false
                }
            })
            .sorted_by_key(|i| OrderedFloat(i.damage_increase_against_with(monster, weapon)))
            .rev()
            .take(3)
            .cloned()
            .collect_vec();
        let best_reduction = equipables
            .iter()
            .filter(|i| i.damage_reduction_against(monster) > 0.0)
            .max_by_key(|i| OrderedFloat(i.damage_reduction_against(monster)))
            .cloned();
        let best_health_increase = equipables
            .iter()
            .filter(|i| i.health() > 0)
            .max_by_key(|i| i.health())
            .cloned();
        if !bests_for_damage.is_empty() {
            bests.extend(bests_for_damage);
        }
        if let Some(best_reduction) = best_reduction {
            bests.push(best_reduction);
        }
        if let Some(best_health_increase) = best_health_increase
            && bests
                .iter()
                .all(|u| u.health() < best_health_increase.health())
        {
            bests.push(best_health_increase);
        }
        bests
            .into_iter()
            .map(|i| Some(i.code.to_owned()))
            .sorted()
            .dedup()
            .collect_vec()
    }

    fn best_utilities_against_with_weapon(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        filter: Filter,
        black_list: Vec<&str>,
    ) -> Vec<Option<String>> {
        let mut upgrades: Vec<Arc<ItemSchema>> = vec![];
        let equipables = self
            .items
            .all()
            .into_iter()
            .filter(|i| {
                !black_list.contains(&i.code.as_str())
                    && char.meets_conditions_for(i)
                    && self.is_eligible(i, filter, char)
            })
            .collect_vec();
        let best_for_damage = equipables
            .iter()
            .filter(|i| i.damage_increase_against_with(monster, weapon) > 0.0)
            .cloned()
            .max_by_key(|i| OrderedFloat(i.damage_increase_against_with(monster, weapon)));
        let best_reduction = equipables
            .iter()
            .filter(|i| i.damage_reduction_against(monster) > 0.0)
            .cloned()
            .max_by_key(|i| OrderedFloat(i.damage_reduction_against(monster)));
        let best_health_increase = equipables
            .iter()
            .filter(|i| i.health() > 0)
            .cloned()
            .max_by_key(|i| i.health());
        let best_restore = equipables
            .iter()
            .filter(|i| i.restore() > 0)
            .cloned()
            .max_by_key(|i| i.restore());
        if let Some(best_for_damage) = best_for_damage {
            upgrades.push(best_for_damage);
        }
        if let Some(best_reduction) = best_reduction {
            upgrades.push(best_reduction);
        }
        if let Some(best_health_increase) = best_health_increase {
            upgrades.push(best_health_increase);
        }
        if let Some(best_restore) = best_restore {
            upgrades.push(best_restore);
        }
        upgrades.sort_by_key(|i| i.code.to_owned());
        upgrades.dedup_by_key(|i| i.code.to_owned());
        upgrades
            .into_iter()
            .map(|i| Some(i.code.to_owned()))
            .collect_vec()
    }

    pub fn best_for_skill(&self, char: &CharacterController, skill: Skill, filter: Filter) -> Gear {
        self.bests_for_skill(char, skill, filter)
            .into_iter()
            .max_set_by_key(|g| g.prospecting())
            .into_iter()
            .max_by_key(|g| g.wisdom())
            .unwrap_or_default()
    }

    pub fn bests_for_skill(
        &self,
        char: &CharacterController,
        skill: Skill,
        filter: Filter,
    ) -> Vec<Gear> {
        let armor_types = [
            Type::Helmet,
            Type::Shield,
            Type::BodyArmor,
            Type::LegArmor,
            Type::Boots,
            Type::Amulet,
        ];
        let mut items = armor_types
            .iter()
            .filter_map(|&item_type| {
                let armors = self.best_armor_for_skill(char, item_type, filter, vec![]);
                (!armors.is_empty()).then_some(
                    armors
                        .iter()
                        .map(|i| ItemWrapper::Armor(i.clone()))
                        .collect_vec(),
                )
            })
            .collect_vec();
        let ring_sets = self.gen_rings_sets_for_skill(char, filter);
        if !ring_sets.is_empty() {
            items.push(ring_sets);
        }
        let artifact_sets = self.gen_artifacts_sets_for_skill(char, filter);
        if !artifact_sets.is_empty() {
            items.push(artifact_sets);
        }
        self.gen_all_gears(self.best_tool(char, skill, filter), items)
    }

    fn best_armor_for_skill(
        &self,
        char: &CharacterController,
        r#type: Type,
        filter: Filter,
        black_list: Vec<String>,
    ) -> Vec<Option<String>> {
        let mut bests: Vec<Arc<ItemSchema>> = vec![];
        let equipables = self
            .items
            .all()
            .into_iter()
            .filter(|i| {
                !black_list.contains(&i.code)
                    && i.r#type() == r#type
                    && char.meets_conditions_for(i)
                    && self.is_eligible(i, filter, char)
                    && (i.wisdom() > 0 || i.prospecting() > 0)
            })
            .collect_vec();
        let best_for_wisdom = equipables.iter().max_by_key(|i| i.wisdom()).cloned();
        let best_for_prospecting = equipables.iter().max_by_key(|i| i.prospecting()).cloned();
        if let Some(best_for_wisdom) = best_for_wisdom {
            bests.push(best_for_wisdom);
        }
        if let Some(best_for_prospecting) = best_for_prospecting {
            bests.push(best_for_prospecting);
        }
        bests
            .iter()
            .map(|i| Some(i.code.to_owned()))
            .sorted()
            .dedup()
            .collect_vec()
    }

    pub fn best_tool(
        &self,
        char: &CharacterController,
        skill: Skill,
        filter: Filter,
    ) -> Option<Arc<ItemSchema>> {
        self.items
            .all()
            .into_iter()
            .filter(|i| {
                i.r#type().is_weapon()
                    && self.is_eligible(i, filter, char)
                    && i.skill_cooldown_reduction(skill) < 0
                    && char.meets_conditions_for(i)
            })
            .min_by_key(|i| i.skill_cooldown_reduction(skill))
    }

    fn gen_rings_sets_for_skill(
        &self,
        char: &CharacterController,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let rings = self.best_armor_for_skill(char, Type::Ring, filter, vec![]);
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
        let rings2 = self.best_armor_for_skill(char, Type::Ring, filter, ring2_black_list);
        let mut ring_sets = [rings, rings2]
            .iter()
            .multi_cartesian_product()
            .map(|rings| [rings[0].clone(), rings[1].clone()])
            .sorted()
            .map(|rings| ItemWrapper::Rings(RingSet::new(rings)))
            .collect_vec();
        ring_sets.dedup();
        ring_sets
    }

    fn is_eligible(&self, i: &ItemSchema, filter: Filter, char: &CharacterController) -> bool {
        if filter.available {
            return char.has_available(&i.code) > 0;
        }
        if i.code == "sanguine_edge_of_rosen" {
            return false;
        }
        if filter.can_craft && i.craft_schema().is_some() && !self.account.can_craft(&i.code) {
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
        //if !filter.from_gift && ITEMS.best_source_of(&i.code).is_some_and(|s| s.is_gift()) {
        //    return false;
        //}
        true
    }

    fn item_from_wrappers(
        &self,
        wrappers: &[&ItemWrapper],
        r#type: Type,
        index: usize,
    ) -> Option<Arc<ItemSchema>> {
        wrappers.iter().find_map(|w| match w {
            ItemWrapper::Armor(Some(armor)) => self
                .items
                .get(armor)
                .and_then(|i| if i.is_of_type(r#type) { Some(i) } else { None }),
            ItemWrapper::Armor(None) => None,
            ItemWrapper::Rings(ring_set) => {
                if let Some(Some(ring)) = ring_set.rings.get(index) {
                    self.items
                        .get(ring)
                        .and_then(|i| if i.is_of_type(r#type) { Some(i) } else { None })
                } else {
                    None
                }
            }
            ItemWrapper::Artifacts(set) => {
                if let Some(Some(artifact)) = set.artifacts.get(index) {
                    self.items
                        .get(artifact)
                        .and_then(|i| if i.is_of_type(r#type) { Some(i) } else { None })
                } else {
                    None
                }
            }
            ItemWrapper::Utility(set) => {
                if let Some(Some(utility)) = set.iter().collect_vec().get(index) {
                    self.items
                        .get(utility)
                        .and_then(|i| if i.is_of_type(r#type) { Some(i) } else { None })
                } else {
                    None
                }
            }
        })
    }

    fn gen_all_gears(
        &self,
        weapon: Option<Arc<ItemSchema>>,
        items: Vec<Vec<ItemWrapper>>,
    ) -> Vec<Gear> {
        items
            .iter()
            .multi_cartesian_product()
            .filter_map(|items| {
                Gear::new(
                    weapon.clone(),
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
            .collect_vec()
    }

    fn gen_artifacts_sets_for_skill(
        &self,
        char: &CharacterController,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let mut artifacts = self.best_armor_for_skill(char, Type::Artifact, filter, vec![]);
        artifacts.push(None);
        let mut sets = [artifacts.clone(), artifacts.clone(), artifacts]
            .iter()
            .multi_cartesian_product()
            .map(|artifacts| {
                [
                    artifacts[0].clone(),
                    artifacts[1].clone(),
                    artifacts[2].clone(),
                ]
            })
            .sorted()
            .filter_map(|artifacts| ArtifactSet::new(artifacts).map(ItemWrapper::Artifacts))
            .collect_vec();
        sets.dedup();
        sets
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Filter {
    pub available: bool,
    pub from_monster: bool,
    pub from_task: bool,
    pub can_craft: bool,
    //pub from_gift: bool,
    pub utilities: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            available: false,
            can_craft: false,
            from_task: true,
            from_monster: true,
            //from_gift: false,
            utilities: false,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ItemWrapper {
    Armor(Option<String>),
    Rings(RingSet),
    Artifacts(ArtifactSet),
    Utility(HashSet<Option<String>>),
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct RingSet {
    rings: [Option<String>; 2],
}

impl RingSet {
    fn new(mut rings: [Option<String>; 2]) -> Self {
        rings.sort();
        RingSet { rings }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ArtifactSet {
    artifacts: [Option<String>; 3],
}

impl ArtifactSet {
    fn new(mut artifacts: [Option<String>; 3]) -> Option<Self> {
        artifacts.sort();
        if artifacts[0]
            .as_ref()
            .is_some_and(|a| artifacts[1].as_ref().is_some_and(|b| a == b))
            || artifacts[1]
                .as_ref()
                .is_some_and(|a| artifacts[2].as_ref().is_some_and(|b| a == b))
            || artifacts[2]
                .as_ref()
                .is_some_and(|a| artifacts[0].as_ref().is_some_and(|b| a == b))
        {
            None
        } else {
            artifacts.sort();
            Some(ArtifactSet { artifacts })
        }
    }
}

#[cfg(test)]
mod tests {
    use artifactsmmo_sdk::{Monsters, models::CharacterSchema};

    use super::*;

    #[test]
    fn best_weapons_against() {
        let gear_finder = GearFinder::default();
        let char = CharacterController::default();
        let data = CharacterSchema {
            level: 30,
            ..Default::default()
        };
        char.update_data(data);

        let weapons = gear_finder.best_weapons_against(
            &char,
            &Monsters::default().get("vampire").unwrap(),
            Default::default(),
        );
        assert_eq!(
            weapons,
            vec![Items::default().get("death_knight_sword").unwrap()]
        );
    }
}
