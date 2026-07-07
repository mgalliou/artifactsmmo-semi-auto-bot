use crate::gear_finder::{ArtifactSet, RingSet, UtilitySet};
use sdk::entities::Item;
use std::cmp::Ordering;

/// A component of a gear setup that fills one or more slots.
///
/// Each variant represents a group of items occupying related slots:
/// - `Armor` — a single armor piece (helmet, shield, body armor, etc.)
/// - `Rings` — a pair of ring slots
/// - `Artifacts` — a triple of artifact slots
/// - `Utility` — a pair of utility slots
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GearComponent {
    Armor(ItemSlot),
    Rings(RingSet),
    Artifacts(ArtifactSet),
    Utility(UtilitySet),
}

impl From<Item> for GearComponent {
    fn from(value: Item) -> Self {
        Self::Armor(ItemSlot::from(value))
    }
}

impl From<&Item> for GearComponent {
    fn from(value: &Item) -> Self {
        Self::from(value.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemSlot(pub Option<Item>);

impl ItemSlot {
    pub const fn slot(&self) -> Option<&Item> {
        self.0.as_ref()
    }
}

impl From<Option<Item>> for ItemSlot {
    fn from(v: Option<Item>) -> Self {
        Self(v)
    }
}

impl From<Item> for ItemSlot {
    fn from(v: Item) -> Self {
        Self(Some(v))
    }
}

impl From<&Item> for ItemSlot {
    fn from(v: &Item) -> Self {
        Self::from(v.clone())
    }
}

impl PartialOrd for ItemSlot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ItemSlot {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

#[cfg(test)]
mod tests {
    use sdk::test_utils::item;
    use super::*;

    #[test]
    fn item_slot_some_less_than_none() {
        let slot = ItemSlot::from(item("copper_ring"));
        let none = ItemSlot(None);
        assert!(slot < none);
    }

    #[test]
    fn item_slot_alphabetical_by_code() {
        let a = ItemSlot::from(item("copper_ring"));
        let b = ItemSlot::from(item("dreadful_ring"));
        assert!(a < b);
    }

    #[test]
    fn item_slot_none_equal_to_none() {
        assert_eq!(ItemSlot(None), ItemSlot(None));
    }

    #[test]
    fn item_slot_some_ne_none() {
        assert_ne!(ItemSlot::from(item("forest_ring")), ItemSlot(None));
    }
}
