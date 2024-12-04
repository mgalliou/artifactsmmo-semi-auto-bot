use artifactsmmo_openapi::models::{CraftSkill, GatheringSkill};
use serde::Deserialize;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    Hash,
    PartialEq,
    Default,
    Deserialize,
    Display,
    AsRefStr,
    EnumIter,
    EnumString,
    EnumIs,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum Skill {
    #[default]
    Combat,
    Mining,
    Woodcutting,
    Fishing,
    Weaponcrafting,
    Gearcrafting,
    Jewelrycrafting,
    Cooking,
    Alchemy,
}

impl Skill {
    pub fn is_gathering(&self) -> bool {
        matches!(
            self,
            Skill::Mining | Skill::Woodcutting | Skill::Fishing | Skill::Alchemy
        )
    }
}

impl From<CraftSkill> for Skill {
    fn from(value: CraftSkill) -> Self {
        match value {
            CraftSkill::Weaponcrafting => Skill::Weaponcrafting,
            CraftSkill::Gearcrafting => Skill::Gearcrafting,
            CraftSkill::Jewelrycrafting => Skill::Jewelrycrafting,
            CraftSkill::Cooking => Skill::Cooking,
            CraftSkill::Woodcutting => Skill::Woodcutting,
            CraftSkill::Mining => Skill::Mining,
            CraftSkill::Alchemy => Skill::Alchemy,
        }
    }
}

impl From<GatheringSkill> for Skill {
    fn from(value: GatheringSkill) -> Self {
        match value {
            GatheringSkill::Woodcutting => Self::Woodcutting,
            GatheringSkill::Mining => Self::Mining,
            GatheringSkill::Fishing => Self::Fishing,
            GatheringSkill::Alchemy => Self::Alchemy,
        }
    }
}
