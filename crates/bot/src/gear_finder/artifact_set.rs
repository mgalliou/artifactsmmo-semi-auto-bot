use crate::gear_finder::item_wrapper::item_cmp;
use sdk::{Slot, entities::Item};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactSet {
    artifacts: [Option<Item>; 3],
}

impl ArtifactSet {
    pub fn new(mut artifacts: [Option<Item>; 3]) -> Option<Self> {
        if artifacts[0].is_some() && artifacts[0] == artifacts[1]
            || artifacts[1].is_some() && artifacts[1] == artifacts[2]
            || artifacts[0].is_some() && artifacts[0] == artifacts[2]
            || (artifacts[0].is_none() && artifacts[1].is_none() && artifacts[2].is_none())
        {
            None
        } else {
            artifacts.sort_by(|a, b| item_cmp(a.as_ref(), b.as_ref()));
            Some(Self { artifacts })
        }
    }

    pub const fn slot(&self, slot: Slot) -> Option<&Item> {
        match slot {
            Slot::Artifact1 => self.artifact1(),
            Slot::Artifact2 => self.artifact2(),
            Slot::Artifact3 => self.artifact3(),
            _ => None,
        }
    }

    pub const fn artifact1(&self) -> Option<&Item> {
        self.artifacts[0].as_ref()
    }

    pub const fn artifact2(&self) -> Option<&Item> {
        self.artifacts[1].as_ref()
    }

    pub const fn artifact3(&self) -> Option<&Item> {
        self.artifacts[2].as_ref()
    }
}

impl Eq for ArtifactSet {}

impl PartialOrd for ArtifactSet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ArtifactSet {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else {
            match item_cmp(self.artifact1(), other.artifact1()) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => match item_cmp(self.artifact2(), other.artifact2()) {
                    Ordering::Less => Ordering::Less,
                    Ordering::Equal => item_cmp(self.artifact3(), other.artifact3()),
                    Ordering::Greater => Ordering::Greater,
                },
                Ordering::Greater => Ordering::Greater,
            }
        }
    }
}
