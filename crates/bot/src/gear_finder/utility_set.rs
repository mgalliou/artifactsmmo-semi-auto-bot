use std::cmp::Ordering;

use crate::gear_finder::item_wrapper::item_cmp;
use sdk::{Slot, entities::Item};

#[derive(Debug, Clone, PartialEq)]
pub struct UtilitySet {
    utilities: [Option<Item>; 2],
}

impl UtilitySet {
    pub fn new(mut utilities: [Option<Item>; 2]) -> Option<Self> {
        if utilities[0].is_some() && utilities[0] == utilities[1]
            || utilities[0].is_none() && utilities[1].is_none()
        {
            None
        } else {
            utilities.sort_by(|a, b| item_cmp(a.as_ref(), b.as_ref()));
            Some(Self { utilities })
        }
    }

    pub const fn slot(&self, slot: Slot) -> Option<&Item> {
        match slot {
            Slot::Utility1 => self.utility1(),
            Slot::Utility2 => self.utility2(),
            _ => None,
        }
    }

    pub const fn utility1(&self) -> Option<&Item> {
        self.utilities[0].as_ref()
    }

    pub const fn utility2(&self) -> Option<&Item> {
        self.utilities[1].as_ref()
    }
}

impl Eq for UtilitySet {}

impl PartialOrd for UtilitySet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UtilitySet {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else {
            match item_cmp(self.utility1(), other.utility1()) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => item_cmp(self.utility2(), other.utility2()),
                Ordering::Greater => Ordering::Greater,
            }
        }
    }
}
