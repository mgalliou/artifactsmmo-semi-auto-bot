use enum_stringify::EnumStringify;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, EnumStringify, EnumIter)]
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
