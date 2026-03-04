use figment::{
    Figment,
    providers::{Format, Toml},
};
use sdk::{models::TaskType, skill::Skill};
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

#[derive(Debug, Default, Clone)]
pub struct BotConfig {
    inner: Arc<RwLock<Arc<BotConfigInner>>>,
}

impl BotConfig {
    pub fn from_file() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Arc::new(BotConfigInner::from_file()))),
        }
    }

    pub fn reload(&self) {
        *self.inner.write().unwrap() = Arc::new(BotConfigInner::from_file())
    }

    pub fn order_gear(&self) -> bool {
        self.inner().order_gear
    }

    pub fn get_char_config(&self, i: usize) -> Option<Arc<CharConfig>> {
        self.inner().get_char_config(i)
    }

    fn inner(&self) -> Arc<BotConfigInner> {
        self.inner.read().unwrap().clone()
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct BotConfigInner {
    pub base_url: String,
    pub token: String,
    pub characters: RwLock<Vec<Arc<CharConfig>>>,
    #[serde(default)]
    pub order_gear: bool,
}

impl BotConfigInner {
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Deserialize, AsRefStr, EnumIter, EnumString, EnumIs,
)]
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
            Self::Orders => {
                write!(f, "progress orders")
            }
            Self::ReachSkillLevel { skill, level } => {
                write!(f, "reach_skill_level: {},{}", skill, level)
            }
            Self::FollowMaxSkillLevel {
                skill,
                skill_to_follow,
            } => {
                write!(f, "follow_max_skill_level: {},{}", skill, skill_to_follow)
            }
            Self::Events => write!(f, "handle events"),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Deserialize, EnumIs)]
pub enum Role {
    #[default]
    Fighter,
    Miner,
    Woodcutter,
    Fisher,
    Weaponcrafter,
}

impl Role {
    pub const fn to_skill(&self) -> Option<Skill> {
        match *self {
            Self::Fighter => None,
            Self::Miner => Some(Skill::Mining),
            Self::Woodcutter => Some(Skill::Woodcutting),
            Self::Fisher => Some(Skill::Fishing),
            Self::Weaponcrafter => Some(Skill::Weaponcrafting),
        }
    }
}
