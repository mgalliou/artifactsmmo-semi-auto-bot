use super::{
    character::Character,
    gear::{Gear, Slot},
    items::{Items, Type},
    ItemSchemaExt,
};
use artifactsmmo_openapi::models::{ItemSchema, MonsterSchema};
use itertools::{Itertools, PeekingNext};
use ordered_float::OrderedFloat;
use std::sync::Arc;

pub struct GearFinder {
    items: Arc<Items>,
}

impl GearFinder {
    pub fn new(items: &Arc<Items>) -> Self {
        Self {
            items: items.clone(),
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
            .max_by_key(|e| OrderedFloat(e.attack_damage_against(monster)))
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
            .flat_map(|w| self.best_against_with_weapon(char, monster, filter, w))
            .collect_vec()
    }

    fn best_against_with_weapon<'a>(
        &'a self,
        char: &Character,
        monster: &MonsterSchema,
        filter: Filter,
        weapon: &'a ItemSchema,
    ) -> Vec<Gear> {
        // TODO: low level gear with empty slots need to be handled properly,
        // Maybe with `Option`s.
        let helmets =
            self.best_armors_against_with_weapon(char, monster, filter, weapon, Type::Helmet);
        let shields =
            self.best_armors_against_with_weapon(char, monster, filter, weapon, Type::Shield);
        let body_armor =
            self.best_armors_against_with_weapon(char, monster, filter, weapon, Type::BodyArmor);
        let leg_armor =
            self.best_armors_against_with_weapon(char, monster, filter, weapon, Type::LegArmor);
        let boots =
            self.best_armors_against_with_weapon(char, monster, filter, weapon, Type::Boots);
        let rings = self.best_armors_against_with_weapon(char, monster, filter, weapon, Type::Ring);
        let mut rings2 = rings.clone();
        if filter == Filter::Available {
            rings2.retain(|i| char.has_available(&i.code) > 1);
        }
        let amulets =
            self.best_armors_against_with_weapon(char, monster, filter, weapon, Type::Amulet);
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
            .collect_vec()
    }

    fn best_armors_against_with_weapon(
        &self,
        char: &Character,
        monster: &MonsterSchema,
        filter: Filter,
        weapon: &ItemSchema,
        r#type: Type,
    ) -> Vec<&ItemSchema> {
        let mut upgrades: Vec<&ItemSchema> = vec![];
        let equipables = self
            .items
            .equipable_at_level(char.level(), r#type)
            .into_iter()
            .filter(|i| Self::is_eligible(i, filter, char))
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
        if let Some(best_for_damage) = best_for_damage {
            upgrades.push(best_for_damage);
        }
        let best_for_resistance = {
            if best_for_damage.is_some() {
                damage_increases
                    .iter()
                    .cloned()
                    .max_by_key(|i| OrderedFloat(i.damage_reduction_against(monster)))
            } else {
                equipables
                    .iter()
                    .cloned()
                    .max_by_key(|i| OrderedFloat(i.damage_reduction_against(monster)))
            }
        };
        if let Some(best_for_resistance) = best_for_resistance {
            upgrades.push(best_for_resistance);
        }
        let best_for_health = {
            if best_for_damage.is_some() {
                damage_increases.into_iter().max_by_key(|i| i.health())
            } else {
                equipables.iter().cloned().max_by_key(|i| i.health())
            }
        };
        if let Some(best_for_health) = best_for_health {
            upgrades.push(best_for_health);
        }
        upgrades
    }

    pub fn best_available_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &MonsterSchema,
    ) -> Gear {
        let gear = char
            .available_equipable_weapons()
            .iter()
            .map(|w| self.best_available_against_with_weapon(char, monster, w))
            .max_by_key(|e| OrderedFloat(e.attack_damage_against(monster)));
        if let Some(gear) = gear {
            return gear;
        }
        char.gear()
    }

    fn best_available_against_with_weapon<'a>(
        &'a self,
        char: &Character,
        monster: &MonsterSchema,
        weapon: &'a ItemSchema,
    ) -> Gear {
        Gear {
            weapon: Some(weapon),
            shield: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Shield,
                monster,
                weapon,
            ),
            helmet: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Helmet,
                monster,
                weapon,
            ),
            body_armor: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::BodyArmor,
                monster,
                weapon,
            ),
            leg_armor: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::LegArmor,
                monster,
                weapon,
            ),
            boots: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Boots,
                monster,
                weapon,
            ),
            ring1: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Ring1,
                monster,
                weapon,
            ),
            ring2: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Ring2,
                monster,
                weapon,
            ),
            amulet: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Amulet,
                monster,
                weapon,
            ),
            artifact1: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Artifact1,
                monster,
                weapon,
            ),
            artifact2: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Artifact2,
                monster,
                weapon,
            ),
            artifact3: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Artifact3,
                monster,
                weapon,
            ),
            utility1: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Utility1,
                monster,
                weapon,
            ),
            utility2: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Utility2,
                monster,
                weapon,
            ),
        }
    }

    /// Returns the best item available for the given `slot` against the given
    /// `monster`, based on item attack damage, damage increase and `monster`
    /// resistances.
    fn best_in_slot_available_against_with_weapon(
        &self,
        char: &Character,
        slot: Slot,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
    ) -> Option<&ItemSchema> {
        match slot {
            Slot::Amulet if char.level() >= 5 && char.level() < 10 => self.items.get("life_amulet"),
            Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Helmet
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet
            | Slot::Boots
            | Slot::Shield => {
                self.best_available_armor_against_with_weapon(char, slot, monster, weapon)
            }
            _ => None,
        }
    }

    /// Returns the best upgrade available in bank or inventory for the given
    /// armor `slot` against the given `monster`, based on the currently equiped
    /// weapon and the `monster` resitances.
    fn best_available_armor_against_with_weapon(
        &self,
        char: &Character,
        slot: Slot,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
    ) -> Option<&ItemSchema> {
        let available = self
            .items
            .equipable_at_level(char.level(), Type::from(slot))
            .into_iter()
            .filter(|i| {
                char.has_available(&i.code) > {
                    if slot.is_ring_2() {
                        1
                    } else {
                        0
                    }
                }
            })
            .collect_vec();
        let mut upgrade = available
            .iter()
            .max_by_key(|i| OrderedFloat(i.damage_increase_against_with(monster, weapon)));
        if upgrade.is_some_and(|i| i.total_damage_increase() <= 0) {
            upgrade = available
                .iter()
                .min_by_key(|i| OrderedFloat(i.damage_from(monster)))
        }
        upgrade.copied()
    }

    fn is_eligible(i: &ItemSchema, filter: Filter, char: &Character) -> bool {
        !i.is_crafted_with("jasper_crystal")
            && !i.is_crafted_with("magical_cure")
            && match filter {
                Filter::All => i.level < 40,
                Filter::Available => char.has_available(&i.code) > 0,
                Filter::Craftable => char.account.can_craft(&i.code),
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
