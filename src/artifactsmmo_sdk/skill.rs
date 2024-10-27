use artifactsmmo_openapi::models::{craft_schema, resource_schema};
use serde::Deserialize;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum Skill {
    Combat,
    Mining,
    Woodcutting,
    Fishing,
    Weaponcrafting,
    Gearcrafting,
    Jewelrycrafting,
    Cooking,
}

impl Skill {
    pub fn is_gathering(&self) -> bool {
        matches!(self, Skill::Mining | Skill::Woodcutting | Skill::Fishing)
    }
}

impl From<craft_schema::Skill> for Skill {
    fn from(value: craft_schema::Skill) -> Self {
        match value {
            craft_schema::Skill::Weaponcrafting => Skill::Weaponcrafting,
            craft_schema::Skill::Gearcrafting => Skill::Gearcrafting,
            craft_schema::Skill::Jewelrycrafting => Skill::Jewelrycrafting,
            craft_schema::Skill::Cooking => Skill::Cooking,
            craft_schema::Skill::Woodcutting => Skill::Woodcutting,
            craft_schema::Skill::Mining => Skill::Mining,
        }
    }
}

impl From<resource_schema::Skill> for Skill {
    fn from(value: resource_schema::Skill) -> Self {
        match value {
            resource_schema::Skill::Woodcutting => Self::Woodcutting,
            resource_schema::Skill::Mining => Self::Mining,
            resource_schema::Skill::Fishing => Self::Fishing,
        }
    }
}
