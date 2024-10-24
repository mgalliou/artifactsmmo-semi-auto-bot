use super::skill::Skill;
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CharConfig {
    #[serde(default)]
    pub idle: bool,
    pub skills: Vec<Skill>,
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
