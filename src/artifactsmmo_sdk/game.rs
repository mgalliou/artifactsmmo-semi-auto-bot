use super::{config::Config, items::Items, maps::Maps, monsters::Monsters, resources::Resources};
use std::sync::Arc;

pub struct Game {
    pub maps: Arc<Maps>,
    pub resources: Arc<Resources>,
    pub monsters: Arc<Monsters>,
    pub items: Arc<Items>,
}

impl Game {
    pub fn new(config: &Config) -> Self {
        let monsters = Arc::new(Monsters::new(config));
        let resources = Arc::new(Resources::new(config));
        Game {
            maps: Arc::new(Maps::new(config)),
            resources: resources.clone(),
            monsters: monsters.clone(),
            items: Arc::new(Items::new(config, resources.clone(), monsters.clone())),
        }
    }
}
