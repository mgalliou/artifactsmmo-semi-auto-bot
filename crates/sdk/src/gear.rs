use crate::{Code, entities::Item, simulator::HasEffects};
use ::std::hash::BuildHasher;
use itertools::Itertools;
use openapi::models::{ItemSlot, SimpleItemSchema};
use std::{
    cell::RefCell,
    collections::HashMap,
    convert::Into,
    default::Default,
    fmt::{self, Display, Formatter},
    mem::swap,
};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Default, Debug, Clone)]
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
    pub(crate) effects_cache: RefCell<HashMap<String, i32>>,
}

impl PartialEq for Gear {
    fn eq(&self, other: &Self) -> bool {
        self.weapon == other.weapon
            && self.helmet == other.helmet
            && self.shield == other.shield
            && self.body_armor == other.body_armor
            && self.leg_armor == other.leg_armor
            && self.boots == other.boots
            && self.amulet == other.amulet
            && self.ring1 == other.ring1
            && self.ring2 == other.ring2
            && self.utility1 == other.utility1
            && self.utility2 == other.utility2
            && self.artifact1 == other.artifact1
            && self.artifact2 == other.artifact2
            && self.artifact3 == other.artifact3
            && self.rune == other.rune
            && self.bag == other.bag
    }
}

impl Eq for Gear {}

impl Gear {
    #[allow(clippy::too_many_arguments)]
    //TODO: return result with invalid gear errors
    #[must_use]
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
    ) -> Option<Self> {
        if utility1.is_some() && utility1 == utility2
            || artifact1.is_some() && artifact1 == artifact2
            || artifact2.is_some() && artifact2 == artifact3
            || artifact1.is_some() && artifact1 == artifact3
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
                rune,
                bag,
                effects_cache: RefCell::new(HashMap::new()),
            })
        }
    }

    #[must_use]
    pub fn with_weapon(mut self, item: Item) -> Self {
        self.weapon = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_helmet(mut self, item: Item) -> Self {
        self.helmet = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_shield(mut self, item: Item) -> Self {
        self.shield = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_body_armor(mut self, item: Item) -> Self {
        self.body_armor = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_leg_armor(mut self, item: Item) -> Self {
        self.leg_armor = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_boots(mut self, item: Item) -> Self {
        self.boots = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_amulet(mut self, item: Item) -> Self {
        self.amulet = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_ring1(mut self, item: Item) -> Self {
        self.ring1 = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_ring2(mut self, item: Item) -> Self {
        self.ring2 = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_utility1(mut self, item: Item) -> Self {
        self.utility1 = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_utility2(mut self, item: Item) -> Self {
        self.utility2 = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_artifact1(mut self, item: Item) -> Self {
        self.artifact1 = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_artifact2(mut self, item: Item) -> Self {
        self.artifact2 = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_artifact3(mut self, item: Item) -> Self {
        self.artifact3 = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_rune(mut self, item: Item) -> Self {
        self.rune = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub fn with_bag(mut self, item: Item) -> Self {
        self.bag = Some(item);
        self.invalidate_cache();
        self
    }

    #[must_use]
    pub const fn item_in(&self, slot: Slot) -> Option<&Item> {
        match slot {
            Slot::Weapon => self.weapon.as_ref(),
            Slot::Shield => self.shield.as_ref(),
            Slot::Helmet => self.helmet.as_ref(),
            Slot::BodyArmor => self.body_armor.as_ref(),
            Slot::LegArmor => self.leg_armor.as_ref(),
            Slot::Boots => self.boots.as_ref(),
            Slot::Ring1 => self.ring1.as_ref(),
            Slot::Ring2 => self.ring2.as_ref(),
            Slot::Amulet => self.amulet.as_ref(),
            Slot::Artifact1 => self.artifact1.as_ref(),
            Slot::Artifact2 => self.artifact2.as_ref(),
            Slot::Artifact3 => self.artifact3.as_ref(),
            Slot::Utility1 => self.utility1.as_ref(),
            Slot::Utility2 => self.utility2.as_ref(),
            Slot::Rune => self.rune.as_ref(),
            Slot::Bag => self.bag.as_ref(),
        }
    }

    pub fn align_to(&mut self, other: &Self) {
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
        self.invalidate_cache();
    }
}

impl Gear {
    fn invalidate_cache(&self) {
        self.effects_cache.borrow_mut().clear();
    }

    fn get_or_compute_effect(&self, effect: &str) -> i32 {
        let mut cache = self.effects_cache.borrow_mut();
        if let Some(&value) = cache.get(effect) {
            return value;
        }
        let value = Slot::iter()
            .map(|slot| self.item_in(slot).map_or(0, |i| i.effect_value(effect)))
            .sum();
        cache.insert(effect.to_owned(), value);
        value
    }
}

impl HasEffects for Gear {
    fn effect_value(&self, effect: &str) -> i32 {
        self.get_or_compute_effect(effect)
    }
}

impl Display for Gear {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for slot in Slot::iter() {
            writeln!(
                f,
                "{slot}: {}",
                self.item_in(slot).map_or("empty", |i| i.code())
            )?;
        }
        Ok(())
    }
}

impl From<Gear> for Vec<SimpleItemSchema> {
    fn from(gear: Gear) -> Self {
        let mut items = Slot::iter()
            .filter_map(|slot| {
                gear.item_in(slot).and_then(|i| {
                    (!slot.is_ring()).then(|| SimpleItemSchema {
                        code: i.code().to_owned(),
                        quantity: slot.max_quantity(),
                    })
                })
            })
            .collect_vec();
        let quantity = if gear.ring1 == gear.ring2 { 2 } else { 1 };
        if let Some(ring1) = gear.ring1 {
            items.push(SimpleItemSchema {
                code: ring1.code().to_owned(),
                quantity,
            });
        }
        if quantity == 1
            && let Some(ring2) = gear.ring2
        {
            items.push(SimpleItemSchema {
                code: ring2.code().to_owned(),
                quantity,
            });
        }
        items
    }
}

impl<S: BuildHasher + Default> From<Gear> for HashMap<String, u32, S> {
    fn from(value: Gear) -> Self {
        Into::<Vec<SimpleItemSchema>>::into(value)
            .iter()
            .map(|i| (i.code().to_owned(), i.quantity))
            .collect()
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
    #[must_use]
    pub const fn max_quantity(&self) -> u32 {
        if self.is_utility() { 100 } else { 1 }
    }

    #[must_use]
    pub const fn is_ring(&self) -> bool {
        self.is_ring_1() || self.is_ring_2()
    }

    #[must_use]
    pub const fn is_artifact(&self) -> bool {
        self.is_artifact_1() || self.is_artifact_2() || self.is_artifact_3()
    }

    #[must_use]
    pub const fn is_utility(&self) -> bool {
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
    use super::*;
    use crate::{client::CollectionClient, test_utils::ITEMS};

    #[test]
    fn check_gear_alignment_is_working() {
        let gear1 = Gear {
            ring1: ITEMS.get("skull_ring"),
            ring2: ITEMS.get("dreadful_ring"),
            utility1: ITEMS.get("minor_health_potion"),
            utility2: ITEMS.get("small_health_potion"),
            artifact1: ITEMS.get("life_crystal"),
            artifact2: ITEMS.get("malefic_crystal"),
            artifact3: ITEMS.get("corrupted_skull"),
            ..Default::default()
        };
        let mut gear2 = Gear {
            ring1: ITEMS.get("dreadful_ring"),
            ring2: ITEMS.get("skull_ring"),
            utility1: ITEMS.get("small_health_potion"),
            utility2: ITEMS.get("minor_health_potion"),
            artifact1: ITEMS.get("malefic_crystal"),
            artifact2: ITEMS.get("corrupted_skull"),
            artifact3: ITEMS.get("life_crystal"),
            ..Default::default()
        };
        let mut gear3 = Gear {
            ring2: ITEMS.get("skull_ring"),
            utility1: ITEMS.get("small_health_potion"),
            artifact2: ITEMS.get("life_crystal"),
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
