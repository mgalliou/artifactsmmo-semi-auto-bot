use artifactsmmo_api_wrapper::ArtifactApi;
use artifactsmmo_openapi::models::{FightSchema, RewardsSchema, SkillDataSchema, SkillInfoSchema};
use fs_extra::file::{read_to_string, write_all};
use game_config::GAME_CONFIG;
use log::error;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::LazyLock};

pub use account::ACCOUNT;
pub use bank::BANK;
pub use fight_simulator::FightSimulator;
pub use game::GAME;
pub use game_config::{CharConfig, GameConfig, Goal};
pub use items::ITEMS;
pub use leveling_helper::LevelingHelper;
pub use maps::MAPS;
pub use monsters::MONSTERS;

pub mod account;
pub mod bank;
pub mod base_bank;
pub mod char;
pub mod consts;
pub mod events;
pub mod fight_simulator;
pub mod game;
pub mod game_config;
pub mod gear;
pub mod gear_finder;
pub mod items;
pub mod leveling_helper;
pub mod maps;
pub mod monsters;
pub mod orderboard;
pub mod resources;
pub mod tasks;
pub mod tasks_rewards;

pub(crate) static API: LazyLock<ArtifactApi> =
    LazyLock::new(|| ArtifactApi::new(&GAME_CONFIG.base_url, &GAME_CONFIG.token));

pub trait PersistedData<D: for<'a> Deserialize<'a> + Serialize> {
    const PATH: &'static str;

    fn retrieve_data() -> D {
        if let Ok(data) = Self::data_from_file::<D>() {
            data
        } else {
            let data = Self::data_from_api();
            if let Err(e) = Self::persist_data(&data) {
                error!("failed to persist data: {}", e);
            }
            data
        }
    }
    fn data_from_api() -> D;
    fn data_from_file<T: for<'a> Deserialize<'a>>() -> Result<T, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&read_to_string(Path::new(
            Self::PATH,
        ))?)?)
    }
    fn persist_data<T: Serialize>(data: T) -> Result<(), Box<dyn std::error::Error>> {
        Ok(write_all(
            Path::new(Self::PATH),
            &serde_json::to_string_pretty(&data)?,
        )?)
    }
    fn refresh_data(&self);
}

pub trait HasDrops {
    fn amount_of(&self, item: &str) -> i32;
}

impl HasDrops for FightSchema {
    fn amount_of(&self, item: &str) -> i32 {
        self.drops
            .iter()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }
}

impl HasDrops for SkillDataSchema {
    fn amount_of(&self, item: &str) -> i32 {
        self.details
            .items
            .iter()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }
}

impl HasDrops for SkillInfoSchema {
    fn amount_of(&self, item: &str) -> i32 {
        self.items
            .iter()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }
}

impl HasDrops for RewardsSchema {
    fn amount_of(&self, item: &str) -> i32 {
        self.items
            .iter()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }
}
