use crate::{Cached, entities::NpcItem};
use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Default, Deref, CollectionClient)]
#[deref(forward)]
#[element(NpcItem)]
pub struct NpcsItemsClient(Arc<NpcsItemsClientInner>);

pub struct NpcsItemsClientInner {
    directory: Box<str>,
    data: ArcSwap<HashMap<String, NpcItem>>,
    fetch: Box<dyn Fn() -> HashMap<String, NpcItem> + Send + Sync>,
}

impl Default for NpcsItemsClientInner {
    fn default() -> Self {
        Self {
            directory: ".cache".into(),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("NpcsItemsClient not initialized")),
        }
    }
}

impl NpcsItemsClient {
    #[must_use]
    pub(crate) fn new(
        path: &str,
        fetch: Box<dyn Fn() -> HashMap<String, NpcItem> + Send + Sync>,
    ) -> Self {
        Self(Arc::new(NpcsItemsClientInner {
            directory: path.into(),
            fetch,
            data: ArcSwap::default(),
        }))
    }

    #[must_use]
    pub fn from_cache(path: &str) -> Self {
        let client = Self::new(
            path,
            Box::new(|| unreachable!("NpcsItemsClient::from_cache has no API fallback")),
        );
        client.init();
        client
    }

    pub fn init(&self) {
        self.data.store(Arc::new(self.fetch()));
        info!("Npcs Items client initilized");
    }
}

impl Cached<HashMap<String, NpcItem>> for NpcsItemsClient {
    const FILE: &'static str = "npcs_items";

    fn directory(&self) -> &str {
        &self.directory
    }

    fn fetch_from_source(&self) -> HashMap<String, NpcItem> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}
