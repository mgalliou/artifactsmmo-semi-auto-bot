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

use super::{
    billboard::Billboard, config::Config, events::Events, items::Items, maps::Maps, monsters::Monsters, resources::Resources
};
use std::sync::{Arc, RwLock};

pub struct Game {
    pub configuration: Configuration,
    pub maps: Arc<Maps>,
    pub resources: Arc<Resources>,
    pub monsters: Arc<Monsters>,
    pub items: Arc<Items>,
    pub events: Arc<Events>,
    pub billboard: Arc<Billboard>,
    pub server_offset: RwLock<TimeDelta>,
}

impl Game {
    pub fn new(config: &Config, billboard: &Arc<Billboard>) -> Self {
        let mut configuration = Configuration::new();
        configuration.base_path = config.base_url.to_owned();
        configuration.bearer_access_token = Some(config.base_url.to_owned());
        let monsters = Arc::new(Monsters::new(config));
        let resources = Arc::new(Resources::new(config));
        let game = Game {
            configuration,
            maps: Arc::new(Maps::new(config)),
            resources: resources.clone(),
            monsters: monsters.clone(),
            items: Arc::new(Items::new(config, resources.clone(), monsters.clone())),
            events: Arc::new(Events::new(config)),
            billboard: billboard.clone(),
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
        let server_time = self.server_time().unwrap();
        let now = Utc::now();
        let _ = self
            .server_offset
            .write()
            .map(|mut so| *so = now - server_time);
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
