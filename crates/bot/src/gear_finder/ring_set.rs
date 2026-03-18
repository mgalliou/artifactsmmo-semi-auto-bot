use std::cmp::Ordering;

use crate::gear_finder::item_wrapper::item_cmp;
use sdk::{Slot, entities::Item};

#[derive(Clone, Debug, PartialEq)]
pub struct RingSet {
    rings: [Option<Item>; 2],
}

impl RingSet {
    pub fn new(mut rings: [Option<Item>; 2]) -> Option<Self> {
        if rings[0].is_none() && rings[1].is_none() {
            None
        } else {
            rings.sort_by(|a, b| item_cmp(a.as_ref(), b.as_ref()));
            Some(Self { rings })
        }
    }

    pub const fn slot(&self, slot: Slot) -> Option<&Item> {
        match slot {
            Slot::Ring1 => self.ring1(),
            Slot::Ring2 => self.ring2(),
            _ => None,
        }
    }

    pub const fn ring1(&self) -> Option<&Item> {
        self.rings[0].as_ref()
    }

    pub const fn ring2(&self) -> Option<&Item> {
        self.rings[1].as_ref()
    }
}

impl Eq for RingSet {}

impl PartialOrd for RingSet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RingSet {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else {
            match item_cmp(self.ring1(), other.ring1()) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => item_cmp(other.ring2(), other.ring2()),
                Ordering::Greater => Ordering::Greater,
            }
        }
    }
}
