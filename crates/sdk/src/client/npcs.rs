use crate::{
    Code, CollectionClient, Data, Persist, client::npcs_items::NpcsItemsClient,
    entities::Npc,
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
#[element(Npc)]
pub struct NpcsClient(Arc<NpcsClientInner>);

#[derive(Default, Debug)]
pub struct NpcsClientInner {
    api: ArtifactApi,
    data: RwLock<Arc<HashMap<String, Npc>>>,
    items: NpcsItemsClient,
}

impl NpcsClient {
    pub(crate) fn new(api: ArtifactApi, items: NpcsItemsClient) -> Self {
        Self(
            NpcsClientInner {
                api,
                data: RwLock::default(),
                items,
            }
            .into(),
        )
    }

    pub fn init(&self) {
        *self.data_mut() = Arc::new(self.load());
        info!("Npcs client initilized");
    }

    #[must_use]
    pub fn items(&self) -> NpcsItemsClient {
        self.items.clone()
    }

    #[must_use]
    pub fn selling(&self, code: &str) -> Vec<Npc> {
        self.items
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
        self.api
            .npcs
            .get_all()
            .unwrap()
            .into_iter()
            .map(|npc| (npc.code.clone(), Npc::new(npc)))
            .collect()
    }

    fn refresh(&self) {
        *self.data_mut() = Arc::new(self.load_from_api());
    }
}


