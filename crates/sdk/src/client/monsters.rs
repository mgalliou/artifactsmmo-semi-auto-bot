use crate::{
    CanProvideXp, CollectionClient, Data, DataEntity, DropsItems, Level, Persist,
    client::events::EventsClient, entities::Monster,
};
use api::ArtifactApi;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
pub struct MonstersClient(Arc<MonstersClientInner>);

#[derive(Default, Debug)]
pub struct MonstersClientInner {
    api: ArtifactApi,
    data: RwLock<Arc<HashMap<String, Monster>>>,
    events: EventsClient,
}

impl MonstersClient {
    pub(crate) fn new(api: ArtifactApi, events: EventsClient) -> Self {
        Self(
            MonstersClientInner {
                api,
                data: RwLock::default(),
                events,
            }
            .into(),
        )
    }

    pub fn init(&self) {
        *self.data_mut() = Arc::new(self.load());
        info!("Monster client initilized");
    }

    #[must_use]
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
            .min_by_key(Level::level)
    }

    pub fn highest_providing_exp(&self, level: u32) -> Option<Monster> {
        self.all()
            .into_iter()
            .filter(|m| m.provides_xp_at(level))
            .max_by_key(Level::level)
    }

    #[must_use]
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
        *self.data_mut() = Arc::new(self.load_from_api());
    }
}

impl DataEntity for MonstersClient {
    type Entity = Monster;
}
