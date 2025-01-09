use super::{
    account::Account, events::Events, fight_simulator::FightSimulator,
    game_config::GameConfig, gear_finder::GearFinder, items::Items,
    leveling_helper::LevelingHelper, maps::Maps, monsters::Monsters, orderboard::OrderBoard,
    resources::Resources, tasks::Tasks,
};
use anyhow::Result;
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
    sync::{Arc, RwLock},
    thread::{sleep, Builder},
    time::Duration,
};

#[derive(Default)]
pub struct Game {
    pub config: Arc<GameConfig>,
    pub server: Arc<Server>,
    pub maps: Arc<Maps>,
    pub resources: Arc<Resources>,
    pub monsters: Arc<Monsters>,
    pub items: Arc<Items>,
    pub events: Arc<Events>,
    pub account: Arc<Account>,
    pub orderboard: Arc<OrderBoard>,
    pub gear_finder: Arc<GearFinder>,
    pub leveling_helper: Arc<LevelingHelper>,
    pub fight_simulator: Arc<FightSimulator>,
}

impl Game {
    pub fn new() -> Self {
        let config = Arc::new(GameConfig::from_file());
        let events = Arc::new(Events::new(&config));
        let monsters = Arc::new(Monsters::new(&config, &events));
        let resources = Arc::new(Resources::new(&config, &events));
        let tasks = Arc::new(Tasks::new(&config));
        let items = Arc::new(Items::new(&config, &resources, &monsters, &tasks));
        let account = Account::new(&config, &items);
        let orderboard = Arc::new(OrderBoard::new(&items, &account));
        let gear_finder = Arc::new(GearFinder::new(&items));
        let maps = Arc::new(Maps::new(&config, &events));
        let leveling_helper = Arc::new(LevelingHelper::new(
            &items, &resources, &monsters, &maps, &account,
        ));
        Game {
            config: config.clone(),
            server: Arc::new(Server::new(&config)),
            maps,
            resources: resources.clone(),
            monsters: monsters.clone(),
            items,
            events,
            account,
            orderboard: orderboard.clone(),
            gear_finder: gear_finder.clone(),
            leveling_helper: leveling_helper.clone(),
            fight_simulator: Arc::new(FightSimulator::new()),
        }
    }

    pub fn init(&self) {
        self.server.update_offset();
        self.account.init_characters(self);
    }

    pub fn run_characters(&self) -> Result<()> {
        for c in self.account.characters() {
            sleep(Duration::from_millis(250));
            Builder::new().spawn(move || {
                c.run_loop();
            })?;
        }
        Ok(())
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

    pub fn status(&self) -> Result<StatusResponseSchema, Error<GetStatusGetError>> {
        get_status_get(&self.configuration)
    }

    pub fn time(&self) -> Option<DateTime<Utc>> {
        let Ok(status) = get_status_get(&self.configuration) else {
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
