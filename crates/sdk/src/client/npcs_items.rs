use crate::{Data, Persist, entities::NpcItem};
use api::ArtifactApi;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(NpcItem)]
pub struct NpcsItemsClient(Arc<NpcsItemsClientInner>);

#[derive(Default, Debug)]
pub struct NpcsItemsClientInner {
    api: ArtifactApi,
    data: RwLock<Arc<HashMap<String, NpcItem>>>,
}

impl NpcsItemsClient {
    pub(crate) fn new(api: ArtifactApi) -> Self {
        Self(
            NpcsItemsClientInner {
                api,
                data: RwLock::default(),
            }
            .into(),
        )
    }

    pub fn init(&self) {
        *self.data_mut() = Arc::new(self.load());
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
        *self.data_mut() = Arc::new(self.load_from_api());
    }
}


