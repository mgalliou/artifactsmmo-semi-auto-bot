use crate::{
    simulator::Simulator,
    items::{DamageType, ItemSchemaExt},
    monsters::MonsterSchemaExt,
};
use artifactsmmo_openapi::models::{ItemSchema, ItemSlot, MonsterSchema, SimpleItemSchema};
use itertools::Itertools;
use std::{fmt::Display, sync::Arc};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Default, Debug, PartialEq)]
pub struct Gear {
    pub weapon: Option<Arc<ItemSchema>>,
    pub helmet: Option<Arc<ItemSchema>>,
    pub shield: Option<Arc<ItemSchema>>,
    pub body_armor: Option<Arc<ItemSchema>>,
    pub leg_armor: Option<Arc<ItemSchema>>,
    pub boots: Option<Arc<ItemSchema>>,
    pub amulet: Option<Arc<ItemSchema>>,
    pub ring1: Option<Arc<ItemSchema>>,
    pub ring2: Option<Arc<ItemSchema>>,
    pub utility1: Option<Arc<ItemSchema>>,
    pub utility2: Option<Arc<ItemSchema>>,
    pub artifact1: Option<Arc<ItemSchema>>,
    pub artifact2: Option<Arc<ItemSchema>>,
    pub artifact3: Option<Arc<ItemSchema>>,
}

impl Gear {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        weapon: Option<Arc<ItemSchema>>,
        helmet: Option<Arc<ItemSchema>>,
        shield: Option<Arc<ItemSchema>>,
        body_armor: Option<Arc<ItemSchema>>,
        leg_armor: Option<Arc<ItemSchema>>,
        boots: Option<Arc<ItemSchema>>,
        amulet: Option<Arc<ItemSchema>>,
        ring1: Option<Arc<ItemSchema>>,
        ring2: Option<Arc<ItemSchema>>,
        utility1: Option<Arc<ItemSchema>>,
        utility2: Option<Arc<ItemSchema>>,
        artifact1: Option<Arc<ItemSchema>>,
        artifact2: Option<Arc<ItemSchema>>,
        artifact3: Option<Arc<ItemSchema>>,
    ) -> Option<Gear> {
        if utility1
            .as_ref()
            .is_some_and(|u1| utility2.as_ref().is_some_and(|u2| u1.code == u2.code))
            || artifact1
                .as_ref()
                .is_some_and(|a1| artifact2.as_ref().is_some_and(|a2| a1.code == a2.code))
            || artifact2
                .as_ref()
                .is_some_and(|a2| artifact3.as_ref().is_some_and(|a3| a2.code == a3.code))
            || artifact1
                .as_ref()
                .is_some_and(|a1| artifact3.as_ref().is_some_and(|a3| a1.code == a3.code))
        {
            None
        } else {
            Some(Self {
                weapon,
                helmet,
                shield,
                body_armor,
                leg_armor,
                boots,
                amulet,
                ring1,
                ring2,
                utility1,
                utility2,
                artifact1,
                artifact2,
                artifact3,
            })
        }
    }

    pub fn attack_damage_against(&self, monster: &MonsterSchema) -> i32 {
        DamageType::iter()
            .map(|t| {
                self.weapon
                    .as_ref()
                    .map_or(0.0, |w| {
                        Simulator::average_dmg(
                            w.attack_damage(t),
                            self.damage_increase(t),
                            monster.resistance(t),
                        )
                    })
                    .round() as i32
            })
            .sum()
    }

    pub fn attack_damage_from(&self, monster: &MonsterSchema) -> i32 {
        DamageType::iter()
            .map(|t| {
                Simulator::average_dmg(monster.attack_damage(t), 0, self.resistance(t)).round()
                    as i32
            })
            .sum()
    }

    // TODO: handle consumables
    fn damage_increase(&self, t: DamageType) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.damage_increase(t)))
            .sum()
    }

    pub fn health_increase(&self) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.health()))
            .sum()
    }

    pub fn slot(&self, slot: Slot) -> Option<Arc<ItemSchema>> {
        match slot {
            Slot::Weapon => self.weapon.clone(),
            Slot::Shield => self.shield.clone(),
            Slot::Helmet => self.helmet.clone(),
            Slot::BodyArmor => self.body_armor.clone(),
            Slot::LegArmor => self.leg_armor.clone(),
            Slot::Boots => self.boots.clone(),
            Slot::Ring1 => self.ring1.clone(),
            Slot::Ring2 => self.ring2.clone(),
            Slot::Amulet => self.amulet.clone(),
            Slot::Artifact1 => self.artifact1.clone(),
            Slot::Artifact2 => self.artifact2.clone(),
            Slot::Artifact3 => self.artifact3.clone(),
            Slot::Utility1 => self.utility1.clone(),
            Slot::Utility2 => self.utility2.clone(),
            _ => None,
        }
    }

    fn resistance(&self, t: DamageType) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.resistance(t)))
            .sum()
    }

    pub fn haste(&self) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.haste()))
            .sum()
    }

    pub fn align_to(&mut self, other: &Gear) {
        if self
            .slot(Slot::Ring1)
            .is_some_and(|r1| other.ring2.as_ref().is_some_and(|r2| r1 == *r2))
            || self
                .slot(Slot::Ring2)
                .is_some_and(|r2| other.ring1.as_ref().is_some_and(|r1| r2 == *r1))
        {
            std::mem::swap(&mut self.ring1, &mut self.ring2);
        }
        if self
            .slot(Slot::Utility1)
            .is_some_and(|u1| other.utility2.as_ref().is_some_and(|u2| u1 == *u2))
            || self
                .slot(Slot::Utility2)
                .is_some_and(|u2| other.utility1.as_ref().is_some_and(|u1| u2 == *u1))
        {
            std::mem::swap(&mut self.utility1, &mut self.utility2);
        }
        if self
            .slot(Slot::Artifact1)
            .is_some_and(|a1| other.artifact2.as_ref().is_some_and(|a2| a1 == *a2))
            || self
                .slot(Slot::Artifact2)
                .is_some_and(|a2| other.artifact1.as_ref().is_some_and(|a1| a2 == *a1))
        {
            std::mem::swap(&mut self.artifact1, &mut self.artifact2);
        }
        if self
            .slot(Slot::Artifact1)
            .is_some_and(|a1| other.artifact3.as_ref().is_some_and(|a3| a1 == *a3))
            || self
                .slot(Slot::Artifact3)
                .is_some_and(|a3| other.artifact1.as_ref().is_some_and(|a1| a3 == *a1))
        {
            std::mem::swap(&mut self.artifact1, &mut self.artifact3);
        }
        if self
            .slot(Slot::Artifact2)
            .is_some_and(|a2| other.artifact3.as_ref().is_some_and(|a3| a2 == *a3))
            || self
                .slot(Slot::Artifact3)
                .is_some_and(|a3| other.artifact2.as_ref().is_some_and(|a2| a3 == *a2))
        {
            std::mem::swap(&mut self.artifact2, &mut self.artifact3);
        }
    }
}

impl Display for Gear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Weapon: {:?}",
            self.weapon.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Shield: {:?}",
            self.shield.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Helmet: {:?}",
            self.helmet.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Body Armor: {:?}",
            self.body_armor.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Leg Armor: {:?}",
            self.leg_armor.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Boots: {:?}",
            self.boots.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Ring 1: {:?}",
            self.ring1.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Ring 2: {:?}",
            self.ring2.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Amulet: {:?}",
            self.amulet.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Artifact 1: {:?}",
            self.artifact1.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Artifact 2: {:?}",
            self.artifact2.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Artifact 3: {:?}",
            self.artifact3.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Consumable 1: {:?}",
            self.utility1.as_ref().map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Consumable 2: {:?}",
            self.utility2.as_ref().map(|w| w.name.to_string())
        )
    }
}

impl From<Gear> for Vec<SimpleItemSchema> {
    fn from(val: Gear) -> Self {
        let mut i = Slot::iter()
            .filter_map(|s| {
                if s.is_ring_1() || s.is_ring_2() {
                    if let Some(item) = val.slot(s) {
                        return Some(SimpleItemSchema {
                            code: item.code.to_owned(),
                            quantity: if s.is_utility_1() || s.is_utility_2() {
                                100
                            } else {
                                1
                            },
                        });
                    }
                }
                None
            })
            .collect_vec();
        match (val.ring1, val.ring2) {
            (Some(r1), Some(r2)) => {
                if r1 == r2 {
                    i.push(SimpleItemSchema {
                        code: r1.code.to_owned(),
                        quantity: 2,
                    })
                } else {
                    i.push(SimpleItemSchema {
                        code: r1.code.to_owned(),
                        quantity: 1,
                    });
                    i.push(SimpleItemSchema {
                        code: r2.code.to_owned(),
                        quantity: 1,
                    });
                }
            }
            (Some(r), None) => i.push(SimpleItemSchema {
                code: r.code.to_owned(),
                quantity: 1,
            }),
            (None, Some(r)) => i.push(SimpleItemSchema {
                code: r.code.to_owned(),
                quantity: 1,
            }),
            (None, None) => (),
        }
        i
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Display, AsRefStr, EnumString, EnumIter, EnumIs,
)]
#[strum(serialize_all = "snake_case")]
pub enum Slot {
    #[default]
    Weapon,
    Shield,
    Helmet,
    BodyArmor,
    LegArmor,
    Boots,
    Ring1,
    Ring2,
    Amulet,
    Artifact1,
    Artifact2,
    Artifact3,
    Utility1,
    Utility2,
    Bag,
    Rune,
}

impl Slot {
    pub fn max_quantity(&self) -> i32 {
        match self {
            Slot::Weapon
            | Slot::Shield
            | Slot::Helmet
            | Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Boots
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet
            | Slot::Artifact1
            | Slot::Artifact2
            | Slot::Artifact3
            | Slot::Bag
            | Slot::Rune => 1,
            Slot::Utility1 => 100,
            Slot::Utility2 => 100,
        }
    }
}

impl From<ItemSlot> for Slot {
    fn from(value: ItemSlot) -> Self {
        match value {
            ItemSlot::Weapon => Self::Weapon,
            ItemSlot::Shield => Self::Shield,
            ItemSlot::Helmet => Self::Helmet,
            ItemSlot::BodyArmor => Self::BodyArmor,
            ItemSlot::LegArmor => Self::LegArmor,
            ItemSlot::Boots => Self::Boots,
            ItemSlot::Ring1 => Self::Ring1,
            ItemSlot::Ring2 => Self::Ring2,
            ItemSlot::Amulet => Self::Amulet,
            ItemSlot::Artifact1 => Self::Artifact1,
            ItemSlot::Artifact2 => Self::Artifact2,
            ItemSlot::Artifact3 => Self::Artifact3,
            ItemSlot::Utility1 => Self::Utility1,
            ItemSlot::Utility2 => Self::Utility2,
            ItemSlot::Bag => Self::Bag,
            ItemSlot::Rune => Self::Rune,
        }
    }
}

impl From<Slot> for ItemSlot {
    fn from(value: Slot) -> Self {
        match value {
            Slot::Weapon => Self::Weapon,
            Slot::Shield => Self::Shield,
            Slot::Helmet => Self::Helmet,
            Slot::BodyArmor => Self::BodyArmor,
            Slot::LegArmor => Self::LegArmor,
            Slot::Boots => Self::Boots,
            Slot::Ring1 => Self::Ring1,
            Slot::Ring2 => Self::Ring2,
            Slot::Amulet => Self::Amulet,
            Slot::Artifact1 => Self::Artifact1,
            Slot::Artifact2 => Self::Artifact2,
            Slot::Artifact3 => Self::Artifact3,
            Slot::Utility1 => Self::Utility1,
            Slot::Utility2 => Self::Utility2,
            Slot::Bag => Self::Bag,
            Slot::Rune => Self::Rune,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ITEMS;

    use super::*;

    #[test]
    fn check_gear_alignment_is_working() {
        let gear1 = Gear {
            ring1: Some(ITEMS.get("skull_ring").unwrap()),
            ring2: Some(ITEMS.get("dreadful_ring").unwrap()),
            utility1: Some(ITEMS.get("minor_health_potion").unwrap()),
            utility2: Some(ITEMS.get("small_health_potion").unwrap()),
            artifact1: Some(ITEMS.get("christmas_star").unwrap()),
            artifact2: Some(ITEMS.get("life_crystal").unwrap()),
            artifact3: Some(ITEMS.get("backpack").unwrap()),
            ..Default::default()
        };
        let mut gear2 = Gear {
            ring1: Some(ITEMS.get("dreadful_ring").unwrap()),
            ring2: Some(ITEMS.get("skull_ring").unwrap()),
            utility1: Some(ITEMS.get("small_health_potion").unwrap()),
            utility2: Some(ITEMS.get("minor_health_potion").unwrap()),
            artifact1: Some(ITEMS.get("life_crystal").unwrap()),
            artifact2: Some(ITEMS.get("backpack").unwrap()),
            artifact3: Some(ITEMS.get("christmas_star").unwrap()),
            ..Default::default()
        };
        let mut gear3 = Gear {
            ring2: Some(ITEMS.get("skull_ring").unwrap()),
            utility1: Some(ITEMS.get("small_health_potion").unwrap()),
            artifact2: Some(ITEMS.get("christmas_star").unwrap()),
            ..Default::default()
        };
        gear2.align_to(&gear1);
        gear3.align_to(&gear1);
        assert_eq!(gear1, gear2);
        assert_eq!(gear3.ring1, gear1.ring1);
        assert_eq!(gear3.utility2, gear1.utility2);
        assert_eq!(gear3.artifact1, gear1.artifact1);
    }
}
