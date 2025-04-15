use account::Account;
use artifactsmmo_api_wrapper::ArtifactApi;
use artifactsmmo_openapi::models::{FightSchema, RewardsSchema, SkillDataSchema, SkillInfoSchema};
use events::Events;
use fs_extra::file::{read_to_string, write_all};
use items::Items;
use log::error;
use maps::Maps;
use monsters::Monsters;
use resources::Resources;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, LazyLock, OnceLock},
};
use tasks_rewards::TasksRewards;

pub use artifactsmmo_openapi::models;
pub use fight_simulator::FightSimulator;

pub mod account;
pub mod base_bank;
pub mod char;
pub mod consts;
pub mod events;
pub mod fight_simulator;
pub mod gear;
pub mod item_code;
pub mod items;
pub mod maps;
pub mod monsters;
pub mod resources;
pub mod server;
pub mod tasks;
pub mod tasks_rewards;

static BASE_URL: OnceLock<String> = OnceLock::new();
static TOKEN: OnceLock<String> = OnceLock::new();
static ACCOUNT_NAME: OnceLock<String> = OnceLock::new();

pub(crate) static API: LazyLock<Arc<ArtifactApi>> = LazyLock::new(|| {
    let base_url = BASE_URL.get_or_init(|| "https://api.artifactsmmo.com".to_owned());
    let token = TOKEN.get_or_init(|| "".to_owned());
    Arc::new(ArtifactApi::new(base_url.to_owned(), token.to_owned()))
});

pub static EVENTS: LazyLock<Arc<Events>> = LazyLock::new(|| Arc::new(Events::new(API.clone())));
pub static RESOURCES: LazyLock<Arc<Resources>> =
    LazyLock::new(|| Arc::new(Resources::new(API.clone(), EVENTS.clone())));
pub static MONSTERS: LazyLock<Arc<Monsters>> =
    LazyLock::new(|| Arc::new(Monsters::new(API.clone(), EVENTS.clone())));
pub static TASKS_REWARDS: LazyLock<Arc<TasksRewards>> =
    LazyLock::new(|| Arc::new(TasksRewards::new(API.clone())));
pub static ITEMS: LazyLock<Items> = LazyLock::new(|| {
    Items::new(
        API.clone(),
        RESOURCES.clone(),
        MONSTERS.clone(),
        TASKS_REWARDS.clone(),
    )
});
pub static MAPS: LazyLock<Maps> = LazyLock::new(|| Maps::new(&API, EVENTS.clone()));
pub static BASE_ACCOUNT: LazyLock<Account> =
    LazyLock::new(|| Account::new(&API, ACCOUNT_NAME.get().unwrap()));

pub fn init(base_url: String, token: String, account_name: String) {
    BASE_URL.get_or_init(|| base_url);
    TOKEN.get_or_init(|| token);
    ACCOUNT_NAME.get_or_init(|| account_name);
}

pub trait PersistedData<D: for<'a> Deserialize<'a> + Serialize> {
    const PATH: &'static str;

    fn retrieve_data(&self) -> D {
        if let Ok(data) = self.data_from_file::<D>() {
            data
        } else {
            let data = self.data_from_api();
            if let Err(e) = Self::persist_data(&data) {
                error!("failed to persist data: {}", e);
            }
            data
        }
    }
    fn data_from_api(&self) -> D;
    fn data_from_file<T: for<'a> Deserialize<'a>>(&self) -> Result<T, Box<dyn std::error::Error>> {
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
