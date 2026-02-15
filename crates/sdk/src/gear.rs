use crate::{Code, entities::Item, simulator::HasEffects};
use openapi::models::{ItemSlot, SimpleEffectSchema, SimpleItemSchema};
use itertools::Itertools;
use std::{fmt::Display, mem::swap};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Gear {
    pub weapon: Option<Item>,
    pub helmet: Option<Item>,
    pub shield: Option<Item>,
    pub body_armor: Option<Item>,
    pub leg_armor: Option<Item>,
    pub boots: Option<Item>,
    pub amulet: Option<Item>,
    pub ring1: Option<Item>,
    pub ring2: Option<Item>,
    pub utility1: Option<Item>,
    pub utility2: Option<Item>,
    pub artifact1: Option<Item>,
    pub artifact2: Option<Item>,
    pub artifact3: Option<Item>,
    pub rune: Option<Item>,
    pub bag: Option<Item>,
}

impl Gear {
    #[allow(clippy::too_many_arguments)]
    //TODO: return result with invalid gear errors
    pub fn new(
        weapon: Option<Item>,
        helmet: Option<Item>,
        shield: Option<Item>,
        body_armor: Option<Item>,
        leg_armor: Option<Item>,
        boots: Option<Item>,
        amulet: Option<Item>,
        ring1: Option<Item>,
        ring2: Option<Item>,
        utility1: Option<Item>,
        utility2: Option<Item>,
        artifact1: Option<Item>,
        artifact2: Option<Item>,
        artifact3: Option<Item>,
        rune: Option<Item>,
        bag: Option<Item>,
    ) -> Option<Gear> {
        (!(utility1.is_some() && utility1 == utility2
            || artifact1.is_some() && artifact1 == artifact2
            || artifact2.is_some() && artifact2 == artifact3
            || artifact1.is_some() && artifact1 == artifact3))
            .then_some(Self {
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
                rune,
                bag,
            })
    }

    pub fn item_in(&self, slot: Slot) -> Option<Item> {
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
            Slot::Rune => self.rune.clone(),
            Slot::Bag => self.bag.clone(),
        }
    }

    pub fn align_to(&mut self, other: &Gear) {
        if self.ring1 == other.ring2 || self.ring2 == other.ring1 {
            swap(&mut self.ring1, &mut self.ring2);
        }
        if self.utility1 == other.utility2 || self.utility2 == other.utility1 {
            swap(&mut self.utility1, &mut self.utility2);
        }
        if self.artifact1 == other.artifact2 || self.artifact2 == other.artifact1 {
            swap(&mut self.artifact1, &mut self.artifact2);
        }
        if self.artifact1 == other.artifact3 || self.artifact3 == other.artifact1 {
            swap(&mut self.artifact1, &mut self.artifact3);
        }
        if self.artifact2 == other.artifact3 || self.artifact3 == other.artifact2 {
            swap(&mut self.artifact2, &mut self.artifact3);
        }
    }
}

impl HasEffects for Gear {
    fn effect_value(&self, effect: &str) -> i32 {
        Slot::iter()
            .map(|s| self.item_in(s).map_or(0, |i| i.effect_value(effect)))
            .sum()
    }

    fn effects(&self) -> Vec<SimpleEffectSchema> {
        vec![]
    }
}

impl Display for Gear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for s in Slot::iter() {
            writeln!(
                f,
                "{}: {}",
                s,
                self.item_in(s).as_ref().map_or("empty", |i| i.code())
            )?;
        }
        Ok(())
    }
}

impl From<Gear> for Vec<SimpleItemSchema> {
    fn from(value: Gear) -> Self {
        let mut items = Slot::iter()
            .filter_map(|slot| {
                value.item_in(slot).and_then(|i| {
                    (!slot.is_ring()).then_some(SimpleItemSchema {
                        code: i.code().to_owned(),
                        quantity: slot.max_quantity(),
                    })
                })
            })
            .collect_vec();
        let mut quantity = 1;
        if value.ring1 == value.ring2 {
            quantity = 2;
        }
        if let Some(ring1) = value.ring1 {
            items.push(SimpleItemSchema {
                code: ring1.code().to_owned(),
                quantity,
            })
        }
        if quantity == 1
            && let Some(ring2) = value.ring2
        {
            items.push(SimpleItemSchema {
                code: ring2.code().to_owned(),
                quantity,
            })
        }
        items
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, Display, AsRefStr, EnumString, EnumIter, EnumIs,
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
    pub fn max_quantity(&self) -> u32 {
        match self.is_utility() {
            true => 100,
            false => 1,
        }
    }

    pub fn is_ring(&self) -> bool {
        self.is_ring_1() || self.is_ring_2()
    }

    pub fn is_artifact(&self) -> bool {
        self.is_artifact_1() || self.is_artifact_2() || self.is_artifact_3()
    }

    pub fn is_utility(&self) -> bool {
        self.is_utility_1() || self.is_utility_2()
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

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, Display, AsRefStr, EnumString, EnumIter, EnumIs,
)]
#[strum(serialize_all = "snake_case")]
pub enum SlotType {
    #[default]
    Weapon,
    Shield,
    Helmet,
    BodyArmor,
    LegArmor,
    Boots,
    Ring,
    Amulet,
    Artifact,
    Utility,
    Bag,
    Rune,
}

impl From<Slot> for SlotType {
    fn from(value: Slot) -> Self {
        match value {
            Slot::Weapon => Self::Weapon,
            Slot::Shield => Self::Shield,
            Slot::Helmet => Self::Helmet,
            Slot::BodyArmor => Self::BodyArmor,
            Slot::LegArmor => Self::LegArmor,
            Slot::Boots => Self::Boots,
            Slot::Ring1 | Slot::Ring2 => Self::Ring,
            Slot::Amulet => Self::Amulet,
            Slot::Artifact1 | Slot::Artifact2 | Slot::Artifact3 => Self::Artifact,
            Slot::Utility1 | Slot::Utility2 => Self::Utility,
            Slot::Bag => Self::Bag,
            Slot::Rune => Self::Rune,
        }
    }
}

#[cfg(test)]
mod tests {
    //TODO: rewrite tests
    // use crate::items::Items;
    //
    // use super::*;
    //
    // #[test]
    // fn check_gear_alignment_is_working() {
    //     let gear1 = Gear {
    //         ring1: Some(ITEMS.get("skull_ring").unwrap()),
    //         ring2: Some(ITEMS.get("dreadful_ring").unwrap()),
    //         utility1: Some(ITEMS.get("minor_health_potion").unwrap()),
    //         utility2: Some(ITEMS.get("small_health_potion").unwrap()),
    //         artifact1: Some(ITEMS.get("christmas_star").unwrap()),
    //         artifact2: Some(ITEMS.get("life_crystal").unwrap()),
    //         artifact3: Some(ITEMS.get("backpack").unwrap()),
    //         ..Default::default()
    //     };
    //     let mut gear2 = Gear {
    //         ring1: Some(ITEMS.get("dreadful_ring").unwrap()),
    //         ring2: Some(ITEMS.get("skull_ring").unwrap()),
    //         utility1: Some(ITEMS.get("small_health_potion").unwrap()),
    //         utility2: Some(ITEMS.get("minor_health_potion").unwrap()),
    //         artifact1: Some(ITEMS.get("life_crystal").unwrap()),
    //         artifact2: Some(ITEMS.get("backpack").unwrap()),
    //         artifact3: Some(ITEMS.get("christmas_star").unwrap()),
    //         ..Default::default()
    //     };
    //     let mut gear3 = Gear {
    //         ring2: Some(ITEMS.get("skull_ring").unwrap()),
    //         utility1: Some(ITEMS.get("small_health_potion").unwrap()),
    //         artifact2: Some(ITEMS.get("christmas_star").unwrap()),
    //         ..Default::default()
    //     };
    //     gear2.align_to(&gear1);
    //     gear3.align_to(&gear1);
    //     assert_eq!(gear1, gear2);
    //     assert_eq!(gear3.ring1, gear1.ring1);
    //     assert_eq!(gear3.utility2, gear1.utility2);
    //     assert_eq!(gear3.artifact1, gear1.artifact1);
    // }
}
