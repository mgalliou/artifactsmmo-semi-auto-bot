use super::skill::Skill;
use serde::Deserialize;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CharConfig {
    #[serde(default)]
    pub idle: bool,
    pub skills: Vec<Skill>,
    pub goals: Vec<Goal>,
    #[serde(default)]
    pub target_monster: Option<String>,
    #[serde(default)]
    pub target_craft: Option<String>,
    #[serde(default)]
    pub target_item: Option<String>,
    #[serde(default)]
    pub do_events: bool,
    #[serde(default)]
    pub do_tasks: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum Goal {
    LevelSkills,
    LevelUp,
}
