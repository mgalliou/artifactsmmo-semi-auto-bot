use artifactsmmo_openapi::models::{craft_schema, resource_schema};
use strum_macros::{AsRefStr, EnumIs, EnumIter, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, AsRefStr, EnumIter, EnumString, EnumIs)]
#[strum(serialize_all = "snake_case")]
pub enum Skill {
    Cooking,
    Fishing,
    Gearcrafting,
    Jewelrycrafting,
    Mining,
    Weaponcrafting,
    Woodcutting,
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
