use crate::{
    CanProvideXp, CollectionClient, DataEntity, DropsItems, Level, Persist,
    client::events::EventsClient, entities::Monster,
};
use api::ArtifactApi;
use itertools::Itertools;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, CollectionClient)]
pub struct MonstersClient {
    data: RwLock<HashMap<String, Monster>>,
    api: Arc<ArtifactApi>,
    events: Arc<EventsClient>,
}

impl MonstersClient {
    pub(crate) fn new(api: Arc<ArtifactApi>, events: Arc<EventsClient>) -> Self {
        let monsters = Self {
            data: Default::default(),
            api,
            events,
        };
        *monsters.data.write().unwrap() = monsters.load();
        monsters
    }

    pub fn dropping(&self, item_code: &str) -> Vec<Monster> {
        self.all()
            .into_iter()
            .filter(|m| m.drops().iter().any(|d| d.code == item_code))
            .collect_vec()
    }

    pub fn lowest_providing_xp_at(&self, level: u32) -> Option<Monster> {
        self.all()
            .into_iter()
            .filter(|m| m.provides_xp_at(level))
            .min_by_key(|m| m.level())
    }

    pub fn highest_providing_exp(&self, level: u32) -> Option<Monster> {
        self.all()
            .into_iter()
            .filter(|m| m.provides_xp_at(level))
            .max_by_key(|m| m.level())
    }

    pub fn is_event(&self, code: &str) -> bool {
        self.events.all().iter().any(|e| e.content().code == code)
    }
}

impl Persist<HashMap<String, Monster>> for MonstersClient {
    const PATH: &'static str = ".cache/monsters.json";

    fn load_from_api(&self) -> HashMap<String, Monster> {
        self.api
            .monsters
            .get_all()
            .unwrap()
            .into_iter()
            .map(|m| (m.code.clone(), Monster::new(m)))
            .collect()
    }

    fn refresh(&self) {
        *self.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for MonstersClient {
    type Entity = Monster;
}
