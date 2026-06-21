use crate::{
    Code, CollectionClient, Persist, client::npcs_items::NpcsItemsClient,
    entities::Npc,
};
use api::ArtifactApi;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use arc_swap::ArcSwap;
use std::{
    collections::HashMap,
    sync::Arc,
};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(Npc)]
pub struct NpcsClient(Arc<NpcsClientInner>);

#[derive(Default, Debug)]
pub struct NpcsClientInner {
    api: ArtifactApi,
    data: ArcSwap<HashMap<String, Npc>>,
    items: NpcsItemsClient,
}

impl NpcsClient {
    pub(crate) fn new(api: ArtifactApi, items: NpcsItemsClient) -> Self {
        Self(
            NpcsClientInner {
                api,
                data: ArcSwap::default(),
                items,
            }
            .into(),
        )
    }

    pub fn init(&self) {
        self.0.data.store(Arc::new(self.load()));
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
        self.0.data.store(Arc::new(self.load_from_api()));
    }
}


