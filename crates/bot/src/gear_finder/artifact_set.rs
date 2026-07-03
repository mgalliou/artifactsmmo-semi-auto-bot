use crate::gear_finder::component::ItemSlot;
use sdk::{Slot, entities::Item};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArtifactSet {
    artifacts: [ItemSlot; 3],
}

impl ArtifactSet {
    pub fn new(artifacts: [Option<Item>; 3]) -> Option<Self> {
        if artifacts[0].is_none() && artifacts[1].is_none() && artifacts[2].is_none() {
            return None;
        }
        let [a, b, c] = artifacts;
        if a.is_some() && a == b || a.is_some() && a == c || b.is_some() && b == c {
            return None;
        }
        let mut slots: [ItemSlot; 3] = [a.into(), b.into(), c.into()];
        slots.sort();
        Some(Self { artifacts: slots })
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
        self.artifacts[0].0.as_ref()
    }

    pub const fn artifact2(&self) -> Option<&Item> {
        self.artifacts[1].0.as_ref()
    }

    pub const fn artifact3(&self) -> Option<&Item> {
        self.artifacts[2].0.as_ref()
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
    fn artifact_set_all_none_returns_none() {
        assert!(ArtifactSet::new([None, None, None]).is_none());
    }

    #[test]
    fn artifact_set_single_artifact() {
        let set = ArtifactSet::new([Some(item("novice_guide")), None, None]).unwrap();
        assert_eq!(set.artifact1(), Some(&item("novice_guide")));
        assert_eq!(set.artifact2(), None);
        assert_eq!(set.artifact3(), None);
    }

    #[test]
    fn artifact_set_three_artifacts_sorted_alphabetically() {
        let set = ArtifactSet::new([
            Some(item("malefic_crystal")),
            Some(item("life_crystal")),
            Some(item("corrupted_skull")),
        ])
        .unwrap();
        assert_eq!(set.artifact1(), Some(&item("corrupted_skull")));
        assert_eq!(set.artifact2(), Some(&item("life_crystal")));
        assert_eq!(set.artifact3(), Some(&item("malefic_crystal")));
    }

    #[test]
    fn artifact_set_none_sorted_last() {
        let set = ArtifactSet::new([None, None, Some(item("novice_guide"))]).unwrap();
        assert_eq!(set.artifact1(), Some(&item("novice_guide")));
        assert_eq!(set.artifact2(), None);
        assert_eq!(set.artifact3(), None);
    }

    #[test]
    fn artifact_set_two_artifacts_none_last() {
        let set = ArtifactSet::new([
            None,
            Some(item("life_crystal")),
            Some(item("corrupted_skull")),
        ])
        .unwrap();
        assert_eq!(set.artifact1(), Some(&item("corrupted_skull")));
        assert_eq!(set.artifact2(), Some(&item("life_crystal")));
        assert_eq!(set.artifact3(), None);
    }

    #[test]
    fn artifact_set_duplicate_a_b_returns_none() {
        let ng = item("novice_guide");
        assert!(ArtifactSet::new([Some(ng.clone()), Some(ng), None]).is_none());
    }

    #[test]
    fn artifact_set_duplicate_a_c_returns_none() {
        let lc = item("life_crystal");
        assert!(ArtifactSet::new([Some(lc.clone()), None, Some(lc)]).is_none());
    }

    #[test]
    fn artifact_set_duplicate_b_c_returns_none() {
        let cs = item("corrupted_skull");
        assert!(ArtifactSet::new([None, Some(cs.clone()), Some(cs)]).is_none());
    }
}
