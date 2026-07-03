use crate::gear_finder::component::ItemSlot;
use sdk::{Slot, entities::Item};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RingSet {
    rings: [ItemSlot; 2],
}

impl RingSet {
    pub fn new(rings: [Option<Item>; 2]) -> Option<Self> {
        if rings[0].is_none() && rings[1].is_none() {
            return None;
        }
        let [a, b] = rings;
        let mut slots: [ItemSlot; 2] = [a.into(), b.into()];
        slots.sort();
        Some(Self { rings: slots })
    }

    pub const fn slot(&self, slot: Slot) -> Option<&Item> {
        match slot {
            Slot::Ring1 => self.ring1(),
            Slot::Ring2 => self.ring2(),
            _ => None,
        }
    }

    pub const fn ring1(&self) -> Option<&Item> {
        self.rings[0].0.as_ref()
    }

    pub const fn ring2(&self) -> Option<&Item> {
        self.rings[1].0.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdk::{CollectionClient, test_utils::ITEMS};

    fn item(code: &str) -> Item {
        ITEMS.get(code).unwrap()
    }

    #[test]
    fn ring_set_both_none_returns_none() {
        assert!(RingSet::new([None, None]).is_none());
    }

    #[test]
    fn ring_set_single_ring() {
        let set = RingSet::new([Some(item("copper_ring")), None]).unwrap();
        assert_eq!(set.ring1(), Some(&item("copper_ring")));
        assert_eq!(set.ring2(), None);
    }

    #[test]
    fn ring_set_two_rings_sorted_alphabetically() {
        let set = RingSet::new([Some(item("iron_ring")), Some(item("forest_ring"))]).unwrap();
        assert_eq!(set.ring1(), Some(&item("forest_ring")));
        assert_eq!(set.ring2(), Some(&item("iron_ring")));
    }

    #[test]
    fn ring_set_none_sorted_last() {
        let set = RingSet::new([None, Some(item("forest_ring"))]).unwrap();
        assert_eq!(set.ring1(), Some(&item("forest_ring")));
        assert_eq!(set.ring2(), None);
    }

    #[test]
    fn ring_set_slot_access() {
        let set = RingSet::new([Some(item("copper_ring")), Some(item("dreadful_ring"))]).unwrap();
        assert_eq!(set.slot(Slot::Ring1), Some(&item("copper_ring")));
        assert_eq!(set.slot(Slot::Ring2), Some(&item("dreadful_ring")));
        assert_eq!(set.slot(Slot::Helmet), None);
        assert_eq!(set.slot(Slot::Amulet), None);
    }
}
