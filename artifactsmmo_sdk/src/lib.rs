use api::ArtifactApi;
use fs_extra::file::{read_to_string, write_all};
use game_config::GAME_CONFIG;
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    path::Path,
    sync::LazyLock,
};

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
pub mod api;
pub mod bank;
pub mod char;
pub mod consts;
pub mod events;
pub mod fight_simulator;
pub mod game;
pub mod game_config;
pub mod gear;
pub mod gear_finder;
pub mod inventory;
pub mod items;
pub mod leveling_helper;
pub mod maps;
pub mod monsters;
pub mod orderboard;
pub mod resources;
pub mod tasks;
pub mod tasks_rewards;

pub static API: LazyLock<ArtifactApi> =
    LazyLock::new(|| ArtifactApi::new(&GAME_CONFIG.base_url, &GAME_CONFIG.token));

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorResponseSchema {
    error: ApiErrorSchema,
}

impl Display for ApiErrorResponseSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.error.message, self.error.code)
    }
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorSchema {
    code: i32,
    message: String,
}

pub trait ApiRequestError {}

pub trait PersistedData<D: for<'a> Deserialize<'a> + Serialize> {
    fn get_data() -> D {
        if let Ok(data) = Self::retreive_data::<D>() {
            data
        } else {
            let data = Self::data_from_api();
            if let Err(e) = Self::persist_data(&data) {
                error!("failed to persist data: {}", e);
            }
            data
        }
    }
    fn path() -> &'static str;
    fn data_from_api() -> D;
    fn retreive_data<T: for<'a> Deserialize<'a>>() -> Result<T, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&read_to_string(Path::new(
            Self::path(),
        ))?)?)
    }
    fn persist_data<T: Serialize>(data: T) -> Result<(), Box<dyn std::error::Error>> {
        Ok(write_all(
            Path::new(Self::path()),
            &serde_json::to_string_pretty(&data)?,
        )?)
    }
}
