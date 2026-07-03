use crate::gear_finder::component::ItemSlot;
use sdk::{Slot, entities::Item};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtilitySet {
    utilities: [ItemSlot; 2],
}

impl UtilitySet {
    pub fn new(utilities: [Option<Item>; 2]) -> Option<Self> {
        if utilities[0].is_none() && utilities[1].is_none() {
            return None;
        }
        let [a, b] = utilities;
        if a.is_some() && a == b {
            return None;
        }
        let mut slots: [ItemSlot; 2] = [a.into(), b.into()];
        slots.sort();
        Some(Self { utilities: slots })
    }

    pub const fn slot(&self, slot: Slot) -> Option<&Item> {
        match slot {
            Slot::Utility1 => self.utility1(),
            Slot::Utility2 => self.utility2(),
            _ => None,
        }
    }

    pub const fn utility1(&self) -> Option<&Item> {
        self.utilities[0].0.as_ref()
    }

    pub const fn utility2(&self) -> Option<&Item> {
        self.utilities[1].0.as_ref()
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
    fn utility_set_both_none_returns_none() {
        assert!(UtilitySet::new([None, None]).is_none());
    }

    #[test]
    fn utility_set_single_utility() {
        let set = UtilitySet::new([Some(item("minor_health_potion")), None]).unwrap();
        assert_eq!(set.utility1(), Some(&item("minor_health_potion")));
        assert_eq!(set.utility2(), None);
    }

    #[test]
    fn utility_set_two_utilities_sorted_alphabetically() {
        let hp = item("health_potion");
        let mhp = item("minor_health_potion");
        let set = UtilitySet::new([Some(mhp), Some(hp)]).unwrap();
        assert_eq!(set.utility1(), Some(&item("health_potion")));
        assert_eq!(set.utility2(), Some(&item("minor_health_potion")));
    }

    #[test]
    fn utility_set_none_sorted_last() {
        let set = UtilitySet::new([None, Some(item("antidote"))]).unwrap();
        assert_eq!(set.utility1(), Some(&item("antidote")));
        assert_eq!(set.utility2(), None);
    }

    #[test]
    fn utility_set_duplicate_returns_none() {
        let hp = item("health_potion");
        assert!(UtilitySet::new([Some(hp.clone()), Some(hp)]).is_none());
    }
}
