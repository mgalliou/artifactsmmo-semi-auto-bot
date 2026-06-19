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

pub fn item_cmp<'a, I>(a: I, b: I) -> Ordering
where
    I: Into<Option<&'a Item>>,
{
    let a = a.into();
    let b = b.into();
    if a == b {
        return Ordering::Equal;
    }
    let Some(a) = a else { return Ordering::Greater };
    let Some(b) = b else { return Ordering::Less };
    a.code().cmp(b.code())
}
