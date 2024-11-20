use super::{
    game_config::GameConfig, events::Events, items::Items, maps::Maps, monsters::Monsters,
    orderboard::OrderBoard, resources::Resources, tasks::Tasks,
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
use log::debug;
use std::sync::{Arc, RwLock};

pub struct Game {
    pub configuration: Configuration,
    pub maps: Arc<Maps>,
    pub resources: Arc<Resources>,
    pub monsters: Arc<Monsters>,
    pub items: Arc<Items>,
    pub events: Arc<Events>,
    pub orderboard: Arc<OrderBoard>,
    pub server_offset: RwLock<TimeDelta>,
}

impl Game {
    pub fn new(config: &GameConfig) -> Self {
        let mut configuration = Configuration::new();
        configuration.base_path = config.base_url.to_owned();
        configuration.bearer_access_token = Some(config.base_url.to_owned());
        let events = Arc::new(Events::new(config));
        let monsters = Arc::new(Monsters::new(config, &events));
        let resources = Arc::new(Resources::new(config, &events));
        let tasks = Arc::new(Tasks::new(config));
        let items = Arc::new(Items::new(config, &resources, &monsters, &tasks));
        let orderboard = Arc::new(OrderBoard::new(&items));
        let game = Game {
            configuration,
            maps: Arc::new(Maps::new(config, &events)),
            resources: resources.clone(),
            monsters: monsters.clone(),
            items,
            events,
            orderboard: orderboard.clone(),
            server_offset: RwLock::new(TimeDelta::default()),
        };
        game.update_offset();
        game
    }

    pub fn server_status(&self) -> Result<StatusResponseSchema, Error<GetStatusGetError>> {
        get_status_get(&self.configuration)
    }

    pub fn server_time(&self) -> Option<DateTime<Utc>> {
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
        let server_time = self.server_time().unwrap_or(Utc::now());
        let now = Utc::now();
        *self.server_offset.write().unwrap() = now - server_time;
        debug!("system time: {}", now);
        debug!("server time: {}", self.server_time().unwrap());
        debug!(
            "time offset: {}s and {}ms",
            self.server_offset.read().unwrap().num_seconds(),
            self.server_offset.read().unwrap().subsec_nanos() / 1000000
        );
        debug!("synced time: {}", now - *self.server_offset.read().unwrap());
    }
}
