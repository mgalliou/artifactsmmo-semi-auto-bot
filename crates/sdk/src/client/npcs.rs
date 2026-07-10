use crate::{Cached, Code, CollectionClient, client::npcs_items::NpcsItemsClient, entities::Npc};
type NpcsSource = Box<dyn Fn() -> HashMap<String, Npc> + Send + Sync + 'static>;

use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(Npc)]
pub struct NpcsClient(Arc<NpcsClientInner>);

pub struct NpcsClientInner {
    cache_dir: Box<str>,
    data: ArcSwap<HashMap<String, Npc>>,
    fetch: NpcsSource,
    items: NpcsItemsClient,
}

impl NpcsClient {
    #[must_use]
    pub(crate) fn new(
        cache_dir: &str,
        fetch: NpcsSource,
        items: NpcsItemsClient,
    ) -> Self {
        Self(Arc::new(NpcsClientInner {
            cache_dir: cache_dir.into(),
            data: ArcSwap::default(),
            fetch,
            items,
        }))
    }

    #[must_use]
    pub fn from_cache(path: &str) -> Self {
        let client = Self::new(
            path,
            Box::new(|| unreachable!("NpcsClient::from_cache has no API fallback")),
            NpcsItemsClient::from_cache(path),
        );
        client.init();
        client
    }

    pub fn init(&self) {
        self.data.store(Arc::new(self.fetch()));
        info!("Npcs client initilized");
    }

    #[must_use]
    pub fn items(&self) -> NpcsItemsClient {
        self.items.clone()
    }

    #[must_use]
    pub fn selling(&self, code: &str) -> Vec<Npc> {
        self.items
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

impl Cached<HashMap<String, Npc>> for NpcsClient {
    const FILE: &'static str = "npcs";

    fn cache_dir(&self) -> &str {
        &self.cache_dir
    }

    fn fetch_from_source(&self) -> HashMap<String, Npc> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}
