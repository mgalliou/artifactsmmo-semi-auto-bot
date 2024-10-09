use std::sync::Arc;

use artifactsmmo_openapi::models::{ItemSchema, MonsterSchema};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use strum::IntoEnumIterator;

use super::{
    average_dmg,
    character::Character,
    equipment::Equipment,
    items::{DamageType, Items, Slot},
    ItemSchemaExt, MonsterSchemaExt,
};

pub struct EquipmentFinder {
    items: Arc<Items>,
}

impl EquipmentFinder {
    pub fn new(items: &Arc<Items>) -> Self {
        Self {
            items: items.clone(),
        }
    }

    pub fn best_available_against<'a>(
        &'a self,
        char: &'a Character,
        monster: &MonsterSchema,
    ) -> Equipment {
        let best_equipment = char
            .available_equipable_weapons()
            .iter()
            .map(|w| self.best_available_against_with_weapon(char, monster, w))
            .max_by_key(|e| OrderedFloat(e.attack_damage_against(monster)));
        if let Some(best_equipment) = best_equipment {
            return best_equipment;
        }
        char.equipment()
    }

    fn best_available_against_with_weapon<'a>(
        &'a self,
        char: &Character,
        monster: &MonsterSchema,
        weapon: &'a ItemSchema,
    ) -> Equipment {
        Equipment {
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
            consumable1: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Consumable1,
                monster,
                weapon,
            ),
            consumable2: self.best_in_slot_available_against_with_weapon(
                char,
                Slot::Consumable2,
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
            .equipable_at_level(char.level(), slot)
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
        let mut upgrade = available.iter().max_by_key(|i| {
            OrderedFloat(Self::armor_attack_damage_against_with_weapon(
                i, monster, weapon,
            ))
        });
        if upgrade.is_some_and(|i| i.total_damage_increase() <= 0) {
            upgrade = available
                .iter()
                .min_by_key(|i| OrderedFloat(i.damage_from(monster)))
        }
        upgrade.copied()
    }

    fn armor_attack_damage_against_with_weapon(
        armor: &ItemSchema,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
    ) -> f32 {
        DamageType::iter()
            .map(|t| {
                average_dmg(
                    weapon.attack_damage(t),
                    armor.damage_increase(t),
                    monster.resistance(t),
                )
            })
            .sum::<f32>()
    }
}
