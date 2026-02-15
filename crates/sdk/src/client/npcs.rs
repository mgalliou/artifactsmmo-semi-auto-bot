use crate::{
    Code, CollectionClient, DataEntity, Persist, client::npcs_items::NpcsItemsClient, entities::Npc,
};
use api::ArtifactApi;
use itertools::Itertools;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, CollectionClient)]
pub struct NpcsClient {
    data: RwLock<HashMap<String, Npc>>,
    api: Arc<ArtifactApi>,
    pub items: Arc<NpcsItemsClient>,
}

impl NpcsClient {
    pub(crate) fn new(api: Arc<ArtifactApi>, items: Arc<NpcsItemsClient>) -> Self {
        let npcs = Self {
            data: Default::default(),
            api,
            items,
        };
        *npcs.data.write().unwrap() = npcs.load();
        npcs
    }

    pub fn selling(&self, code: &str) -> Vec<Npc> {
        self.items
            .all()
            .iter()
            .filter(|i| i.is_buyable() && i.code() == code)
            .flat_map(|i| self.get(i.npc_code()))
            .collect_vec()
    }
}

impl Persist<HashMap<String, Npc>> for NpcsClient {
    const PATH: &'static str = ".cache/npcs.json";

    fn load_from_api(&self) -> HashMap<String, Npc> {
        self.api
            .npcs
            .get_all()
            .unwrap()
            .into_iter()
            .map(|npc| (npc.code.clone(), Npc::new(npc)))
            .collect()
    }

    fn refresh(&self) {
        *self.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for NpcsClient {
    type Entity = Npc;
}
