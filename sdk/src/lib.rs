use artifactsmmo_api_wrapper::ArtifactApi;
use artifactsmmo_openapi::models::{FightSchema, RewardsSchema, SkillDataSchema, SkillInfoSchema};
use fs_extra::file::{read_to_string, write_all};
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{LazyLock, OnceLock},
};

pub use artifactsmmo_openapi::models;
pub use fight_simulator::FightSimulator;
pub use items::ITEMS;
pub use maps::MAPS;
pub use monsters::MONSTERS;

pub mod base_bank;
pub mod char;
pub mod consts;
pub mod events;
pub mod fight_simulator;
pub mod gear;
pub mod items;
pub mod maps;
pub mod monsters;
pub mod resources;
pub mod server;
pub mod tasks;
pub mod tasks_rewards;

static BASE_URL: OnceLock<String> = OnceLock::new();
static TOKEN: OnceLock<String> = OnceLock::new();

pub(crate) static API: LazyLock<ArtifactApi> = LazyLock::new(|| {
    let Some(base_url) = BASE_URL.get() else {
        panic!("SDK not initialized");
    };
    let Some(token) = TOKEN.get() else {
        panic!("SDK not initialized");
    };
    ArtifactApi::new(base_url.to_owned(), token.to_owned())
});

pub fn init(base_url: &str, token: &str) {
    BASE_URL.get_or_init(|| base_url.to_string());
    TOKEN.get_or_init(|| token.to_string());
}

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
