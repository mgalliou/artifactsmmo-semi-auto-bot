use crate::gear_finder::{ArtifactSet, RingSet, UtilitySet};
use sdk::{Code, entities::Item};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq)]
pub enum ItemWrapper {
    Armor(Option<Item>),
    Rings(RingSet),
    Artifacts(ArtifactSet),
    Utility(UtilitySet),
}

impl From<Item> for ItemWrapper {
    fn from(value: Item) -> Self {
        Self::Armor(Some(value))
    }
}

impl From<&Item> for ItemWrapper {
    fn from(value: &Item) -> Self {
        Self::Armor(Some(value.clone()))
    }
}

impl From<RingSet> for ItemWrapper {
    fn from(value: RingSet) -> Self {
        Self::Rings(value)
    }
}

pub fn item_cmp(a: Option<&Item>, b: Option<&Item>) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let Some(a) = a else { return Ordering::Greater };
    let Some(b) = b else { return Ordering::Less };
    a.code().cmp(b.code())
}
