use artifactsmmo_openapi::models::craft_schema;
use enum_stringify::EnumStringify;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, EnumStringify, EnumIter)]
#[enum_stringify(case = "lower")]
pub enum Skill {
    Cooking,
    Fishing,
    Gearcrafting,
    Jewelrycrafting,
    Mining,
    Weaponcrafting,
    Woodcutting,
}

impl Skill {
    pub fn from_craft_schema_skill(skill: craft_schema::Skill) -> Self {
        match skill {
            craft_schema::Skill::Weaponcrafting => Skill::Weaponcrafting,
            craft_schema::Skill::Gearcrafting => Skill::Gearcrafting,
            craft_schema::Skill::Jewelrycrafting => Skill::Jewelrycrafting,
            craft_schema::Skill::Cooking => Skill::Cooking,
            craft_schema::Skill::Woodcutting => Skill::Woodcutting,
            craft_schema::Skill::Mining => Skill::Mining,
        }
    }
}
