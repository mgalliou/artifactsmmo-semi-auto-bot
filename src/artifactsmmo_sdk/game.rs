use super::{
    account::Account, events::Events, game_config::GameConfig, items::Items, maps::Maps,
    monsters::Monsters, orderboard::OrderBoard, resources::Resources, tasks::Tasks,
};
use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        default_api::{get_status_get, GetStatusGetError},
        Error,
    },
    models::StatusResponseSchema,
};
use chrono::{DateTime, TimeDelta, Utc};
use figment::{
    providers::{Format, Toml},
    Figment,
};
use log::debug;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct Game {
    pub configuration: Configuration,
    pub maps: Arc<Maps>,
    pub resources: Arc<Resources>,
    pub monsters: Arc<Monsters>,
    pub items: Arc<Items>,
    pub events: Arc<Events>,
    pub account: Arc<Account>,
    pub orderboard: Arc<OrderBoard>,
    pub server: Arc<Server>,
}

impl Game {
    pub fn new() -> Self {
        let config: Arc<GameConfig> = Arc::new(
            Figment::new()
                .merge(Toml::file_exact("ArtifactsMMO.toml"))
                .extract()
                .unwrap(),
        );
        let mut configuration = Configuration::new();
        configuration.base_path = config.base_url.to_owned();
        configuration.bearer_access_token = Some(config.base_url.to_owned());
        let events = Arc::new(Events::new(&config));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let resources = Arc::new(Resources::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));
        let account = Account::new(&config, &items);
        let orderboard = Arc::new(OrderBoard::new(&items, &account));
        Game {
            configuration,
            maps: Arc::new(Maps::new(&config, &events)),
            resources: resources.clone(),
            monsters: monsters.clone(),
            items,
            events,
            orderboard: orderboard.clone(),
            account,
            server: Arc::new(Server::new(&config)),
        }
    }

    pub fn init(&self) {
        self.server.update_offset();
        self.account.init_characters(self);
        self.events.refresh();
    }
}

#[derive(Default)]
pub struct Server {
    pub configuration: Configuration,
    pub server_offset: RwLock<TimeDelta>,
}

impl Server {
    pub fn new(config: &GameConfig) -> Self {
        let mut conf = Configuration::new();
        conf.base_path = config.base_url.to_owned();
        Self {
            configuration: conf,
            server_offset: RwLock::new(TimeDelta::default()),
        }
    }

    pub fn server_status(&self) -> Result<StatusResponseSchema, Error<GetStatusGetError>> {
        get_status_get(&self.configuration)
    }

    pub fn time(&self) -> Option<DateTime<Utc>> {
        match get_status_get(&self.configuration) {
            Ok(s) => match DateTime::parse_from_rfc3339(&s.data.server_time) {
                Ok(t) => Some(t.to_utc()),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }

    pub fn update_offset(&self) {
        // TODO: properly handle failure to retreive server_time
        let server_time = self.time().unwrap_or(Utc::now());
        let now = Utc::now();
        *self.server_offset.write().unwrap() = now - server_time;
        debug!("system time: {}", now);
        debug!("server time: {}", self.time().unwrap());
        debug!(
            "time offset: {}s and {}ms",
            self.server_offset.read().unwrap().num_seconds(),
            self.server_offset.read().unwrap().subsec_nanos() / 1000000
        );
        debug!("synced time: {}", now - *self.server_offset.read().unwrap());
    }
}
