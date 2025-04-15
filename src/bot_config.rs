use artifactsmmo_sdk::{char::Skill, models::TaskType};
use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::Display,
    sync::{LazyLock, RwLock},
};
use strum_macros::{AsRefStr, EnumIs, EnumIter, EnumString};

pub static BOT_CONFIG: LazyLock<BotConfig> = LazyLock::new(BotConfig::from_file);

#[derive(Debug, Default, Deserialize)]
pub struct BotConfig {
    pub base_url: String,
    pub token: String,
    pub account_name: String,
    pub characters: Vec<RwLock<CharConfig>>,
}

impl BotConfig {
    fn from_file() -> Self {
        Figment::new()
            .merge(Toml::file_exact("ArtifactsMMO.toml"))
            .extract()
            .unwrap()
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CharConfig {
    #[serde(default)]
    pub idle: bool,
    pub skills: HashSet<Skill>,
    pub task_type: TaskType,
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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, AsRefStr, EnumIter, EnumString, EnumIs)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum Goal {
    Orders,
    ReachSkillLevel {
        skill: Skill,
        level: i32,
    },
    FollowMaxSkillLevel {
        skill: Skill,
        skill_to_follow: Skill,
    },
    Events,
}

impl Display for Goal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Goal::Orders => {
                write!(f, "progress orders")
            }
            Goal::ReachSkillLevel { skill, level } => {
                write!(f, "reach_skill_level: {},{}", skill, level)
            }
            Goal::FollowMaxSkillLevel {
                skill,
                skill_to_follow,
            } => {
                write!(f, "follow_max_skill_level: {},{}", skill, skill_to_follow)
            }
            Goal::Events => write!(f, "handle events"),
        }
    }
}
