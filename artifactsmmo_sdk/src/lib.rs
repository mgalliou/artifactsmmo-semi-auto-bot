use fs_extra::file::{read_to_string, write_all};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    path::Path,
};

pub use account::ACCOUNT;
pub use bank::BANK;
pub use fight_simulator::FightSimulator;
pub use game::GAME;
pub use items::ITEMS;
pub use game_config::{CharConfig, GameConfig, Goal};
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

pub fn retreive_data<T: for<'a> Deserialize<'a>>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&read_to_string(path)?)?)
}

pub fn persist_data<T: Serialize>(data: T, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(write_all(path, &serde_json::to_string_pretty(&data)?)?)
}
