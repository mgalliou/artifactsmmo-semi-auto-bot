use crate::{Cached, entities::NpcItem};
type NpcsItemsSource = Box<dyn Fn() -> HashMap<String, NpcItem> + Send + Sync + 'static>;

use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(NpcItem)]
pub struct NpcsItemsClient(Arc<NpcsItemsClientInner>);

pub struct NpcsItemsClientInner {
    cache_dir: Box<str>,
    data: ArcSwap<HashMap<String, NpcItem>>,
    fetch: NpcsItemsSource,
}

impl NpcsItemsClient {
    #[must_use]
    pub(crate) fn new(cache_dir: &str, fetch: NpcsItemsSource) -> Self {
        Self(Arc::new(NpcsItemsClientInner {
            cache_dir: cache_dir.into(),
            data: ArcSwap::default(),
            fetch,
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

    fn cache_dir(&self) -> &str {
        &self.cache_dir
    }

    fn fetch_from_source(&self) -> HashMap<String, NpcItem> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}
