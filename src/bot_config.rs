use artifactsmmo_sdk::{char::Skill, models::TaskType};
use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::Display,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};
use strum_macros::{AsRefStr, EnumIs, EnumIter, EnumString};

#[derive(Debug, Default)]
pub struct BotConfig {
    inner: RwLock<Arc<BaseBotConfig>>,
}

impl BotConfig {
    pub fn from_file() -> Self {
        Self {
            inner: RwLock::new(Arc::new(BaseBotConfig::from_file())),
        }
    }

    pub fn reload(&self) {
        *self.inner.write().unwrap() = Arc::new(BaseBotConfig::from_file())
    }

    pub fn order_gear(&self) -> bool {
        self.inner().order_gear
    }

    pub fn get_char_config(&self, i: usize) -> Option<Arc<CharConfig>> {
        self.inner().get_char_config(i)
    }

    fn inner(&self) -> Arc<BaseBotConfig> {
        self.inner.read().unwrap().clone()
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct BaseBotConfig {
    pub base_url: String,
    pub token: String,
    pub characters: RwLock<Vec<Arc<CharConfig>>>,
    #[serde(default)]
    pub order_gear: bool,
}

impl BaseBotConfig {
    pub fn from_file() -> Self {
        Figment::new()
            .merge(Toml::file_exact("ArtifactsMMO.toml"))
            .extract()
            .unwrap()
    }

    pub fn get_char_config(&self, i: usize) -> Option<Arc<CharConfig>> {
        self.characters.read().unwrap().get(i).cloned()
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct CharConfig {
    #[serde(default)]
    idle: AtomicBool,
    #[serde(default)]
    is_trader: AtomicBool,
    #[serde(default)]
    pub task_type: TaskType,
    #[serde(default)]
    skills: RwLock<HashSet<Skill>>,
    #[serde(default)]
    pub goals: Vec<Goal>,
}

impl CharConfig {
    pub fn toggle_idle(&self) {
        self.idle.fetch_not(Ordering::SeqCst);
    }

    pub fn is_idle(&self) -> bool {
        self.idle.load(Ordering::SeqCst)
    }

    pub fn is_trader(&self) -> bool {
        self.is_trader.load(Ordering::SeqCst)
    }

    pub fn skill_is_enabled(&self, skill: Skill) -> bool {
        self.skills.write().unwrap().contains(&skill)
    }

    pub fn disable_skill(&self, skill: Skill) {
        self.skills.write().unwrap().remove(&skill);
    }

    pub fn enable_skill(&self, skill: Skill) {
        self.skills.write().unwrap().insert(skill);
    }

    pub fn skills(&self) -> HashSet<Skill> {
        (*self.skills.read().unwrap()).clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, AsRefStr, EnumIter, EnumString, EnumIs)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum Goal {
    Orders,
    ReachSkillLevel {
        skill: Skill,
        level: u32,
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

#[derive(Debug, Default, PartialEq, Copy, Clone, Deserialize, EnumIs)]
pub enum Role {
    #[default]
    Fighter,
    Miner,
    Woodcutter,
    Fisher,
    Weaponcrafter,
}

impl Role {
    pub fn to_skill(&self) -> Option<Skill> {
        match *self {
            Role::Fighter => None,
            Role::Miner => Some(Skill::Mining),
            Role::Woodcutter => Some(Skill::Woodcutting),
            Role::Fisher => Some(Skill::Fishing),
            Role::Weaponcrafter => Some(Skill::Weaponcrafting),
        }
    }
}
