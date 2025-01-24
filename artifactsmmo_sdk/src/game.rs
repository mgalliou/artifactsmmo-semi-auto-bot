use crate::{account::ACCOUNT, game_config::GAME_CONFIG};
use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        default_api::{get_status_get, GetStatusGetError},
        Error,
    },
    models::StatusResponseSchema,
};
use chrono::{DateTime, TimeDelta, Utc};
use log::{debug, error};
use std::{
    sync::{LazyLock, RwLock},
    thread::{sleep, Builder},
    time::Duration,
};

pub static GAME: LazyLock<Game> = LazyLock::new(Game::new);

pub struct Game {}

impl Game {
    fn new() -> Self {
        Game {}
    }

    pub fn run_characters(&self) {
        for c in ACCOUNT.characters() {
            sleep(Duration::from_millis(250));
            if let Err(e) = Builder::new().spawn(move || {
                c.run_loop();
            }) {
                error!("failed to spawn character thread: {}", e);
            }
        }
    }
}

pub static SERVER: LazyLock<Server> = LazyLock::new(Server::new);

#[derive(Default)]
pub struct Server {
    pub configuration: Configuration,
    pub server_offset: RwLock<TimeDelta>,
}

impl Server {
    fn new() -> Self {
        let mut conf = Configuration::new();
        conf.base_path = GAME_CONFIG.base_url.to_owned();
        let server = Self {
            configuration: conf,
            server_offset: RwLock::new(TimeDelta::default()),
        };
        server.update_offset();
        server
    }

    pub fn status(&self) -> Result<StatusResponseSchema, Error<GetStatusGetError>> {
        get_status_get(&self.configuration)
    }

    pub fn time(&self) -> Option<DateTime<Utc>> {
        let Ok(status) = self.status() else {
            return None;
        };
        let Ok(time) = DateTime::parse_from_rfc3339(&status.data.server_time) else {
            return None;
        };
        Some(time.to_utc())
    }

    pub fn update_offset(&self) {
        let now = Utc::now();
        let Some(server_time) = self.time() else {
            error!("failed to update time offset");
            return;
        };
        let round_trip = now - Utc::now();
        *self.server_offset.write().unwrap() = now - server_time + (round_trip / 2);
        debug!("system time: {}", now);
        debug!("server time: {}", server_time);
        debug!(
            "time offset: {}s and {}ms",
            self.server_offset.read().unwrap().num_seconds(),
            self.server_offset.read().unwrap().subsec_nanos() / 1000000
        );
        debug!("synced time: {}", now - *self.server_offset.read().unwrap());
    }
}
