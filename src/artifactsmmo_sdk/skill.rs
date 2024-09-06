use artifactsmmo_openapi::models::craft_schema;
use strum_macros::{AsRefStr, EnumIter, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, AsRefStr, EnumIter, EnumString)]
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
