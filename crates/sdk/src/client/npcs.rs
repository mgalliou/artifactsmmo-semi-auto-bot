use crate::{
    Code, CollectionClient, DataEntity, Persist, client::npcs_items::NpcsItemsClient, entities::Npc,
};
use api::ArtifactApi;
use itertools::Itertools;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, CollectionClient)]
pub struct NpcsClient(Arc<NpcsClientInner>);

#[derive(Default, Debug)]
pub struct NpcsClientInner {
    api: ArtifactApi,
    data: RwLock<HashMap<String, Npc>>,
    items: NpcsItemsClient,
}

impl NpcsClient {
    pub(crate) fn new(api: ArtifactApi, items: NpcsItemsClient) -> Self {
        let npcs = Self(
            NpcsClientInner {
                api,
                data: RwLock::default(),
                items,
            }
            .into(),
        );
        *npcs.0.data.write().unwrap() = npcs.load();
        npcs
    }

    pub fn items(&self) -> NpcsItemsClient {
        self.0.items.clone()
    }

    pub fn selling(&self, code: &str) -> Vec<Npc> {
        self.0
            .items
            .all()
            .iter()
            .filter_map(|i| {
                if i.is_buyable() && i.code() == code {
                    self.get(i.npc_code())
                } else {
                    None
                }
            })
            .collect_vec()
    }
}

impl Persist<HashMap<String, Npc>> for NpcsClient {
    const PATH: &'static str = ".cache/npcs.json";

    fn load_from_api(&self) -> HashMap<String, Npc> {
        self.0
            .api
            .npcs
            .get_all()
            .unwrap()
            .into_iter()
            .map(|npc| (npc.code.clone(), Npc::new(npc)))
            .collect()
    }

    fn refresh(&self) {
        *self.0.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for NpcsClient {
    type Entity = Npc;
}
