use crate::{Cached, entities::NpcItem};
use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Deref, Default, CollectionClient)]
#[deref(forward)]
#[element(NpcItem)]
pub struct NpcsItemsClient(Arc<NpcsItemsClientInner>);

pub struct NpcsItemsClientInner {
    path: Box<str>,
    data: ArcSwap<HashMap<String, NpcItem>>,
    fetch: Box<dyn Fn() -> HashMap<String, NpcItem> + Send + Sync>,
}

impl Default for NpcsItemsClientInner {
    fn default() -> Self {
        Self {
            path: ".cache/npcs_items.ron".into(),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("NpcsItemsClient not initialized")),
        }
    }
}

impl NpcsItemsClient {
    pub(crate) fn new(
        path: &str,
        fetch: Box<dyn Fn() -> HashMap<String, NpcItem> + Send + Sync>,
    ) -> Self {
        Self(
            NpcsItemsClientInner {
                path: path.into(),
                fetch,
                data: ArcSwap::default(),
            }
            .into(),
        )
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
    fn path(&self) -> &str {
        &self.path
    }

    fn fetch_from_source(&self) -> HashMap<String, NpcItem> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}
