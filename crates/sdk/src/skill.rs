use openapi::models::{CraftSkill, GatheringSkill};
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
    pub const fn is_gathering(&self) -> bool {
        matches!(
            self,
            Self::Mining | Self::Woodcutting | Self::Fishing | Self::Alchemy
        )
    }
}

impl From<CraftSkill> for Skill {
    fn from(value: CraftSkill) -> Self {
        match value {
            CraftSkill::Weaponcrafting => Self::Weaponcrafting,
            CraftSkill::Gearcrafting => Self::Gearcrafting,
            CraftSkill::Jewelrycrafting => Self::Jewelrycrafting,
            CraftSkill::Cooking => Self::Cooking,
            CraftSkill::Woodcutting => Self::Woodcutting,
            CraftSkill::Mining => Self::Mining,
            CraftSkill::Alchemy => Self::Alchemy,
        }
    }
}

impl From<GatheringSkill> for Skill {
    fn from(value: GatheringSkill) -> Self {
        match value {
            GatheringSkill::Mining => Self::Mining,
            GatheringSkill::Woodcutting => Self::Woodcutting,
            GatheringSkill::Fishing => Self::Fishing,
            GatheringSkill::Alchemy => Self::Alchemy,
        }
    }
}
