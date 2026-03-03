use crate::{DataEntity, Persist, entities::NpcItem};
use api::ArtifactApi;
use sdk_derive::CollectionClient;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, CollectionClient)]
pub struct NpcsItemsClient(Arc<NpcsItemsClientInner>);

#[derive(Default, Debug)]
pub struct NpcsItemsClientInner {
    data: RwLock<HashMap<String, NpcItem>>,
    api: Arc<ArtifactApi>,
}

impl NpcsItemsClient {
    pub(crate) fn new(api: Arc<ArtifactApi>) -> Self {
        let npcs_items = Self(Arc::new(NpcsItemsClientInner {
            data: Default::default(),
            api,
        }));
        *npcs_items.0.data.write().unwrap() = npcs_items.load();
        npcs_items
    }
}

impl Persist<HashMap<String, NpcItem>> for NpcsItemsClient {
    const PATH: &'static str = ".cache/npcs_items.json";

    fn load_from_api(&self) -> HashMap<String, NpcItem> {
        self.0.api
            .npcs
            .get_items()
            .unwrap()
            .into_iter()
            .map(|npc| (npc.code.clone(), NpcItem::new(npc)))
            .collect()
    }

    fn refresh(&self) {
        *self.0.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for NpcsItemsClient {
    type Entity = NpcItem;
}
