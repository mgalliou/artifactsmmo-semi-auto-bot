use crate::{Persist, entities::NpcItem};
use api::ArtifactApi;
use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{collections::HashMap, sync::Arc};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(NpcItem)]
pub struct NpcsItemsClient(Arc<NpcsItemsClientInner>);

#[derive(Default, Debug)]
pub struct NpcsItemsClientInner {
    api: ArtifactApi,
    data: ArcSwap<HashMap<String, NpcItem>>,
}

impl NpcsItemsClient {
    pub(crate) fn new(api: ArtifactApi) -> Self {
        Self(
            NpcsItemsClientInner {
                api,
                data: ArcSwap::default(),
            }
            .into(),
        )
    }

    pub fn init(&self) {
        self.0.data.store(Arc::new(self.load()));
        info!("Npcs Items client initilized");
    }
}

impl Persist<HashMap<String, NpcItem>> for NpcsItemsClient {
    const PATH: &'static str = ".cache/npcs_items.json";

    fn load_from_api(&self) -> HashMap<String, NpcItem> {
        self.api
            .npcs
            .get_items()
            .unwrap()
            .into_iter()
            .map(|npc| (npc.code.clone(), NpcItem::new(npc)))
            .collect()
    }

    fn refresh(&self) {
        self.0.data.store(Arc::new(self.load_from_api()));
    }
}
