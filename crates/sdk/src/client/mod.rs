use api::ArtifactApi;
use derive_more::Deref;
use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::Arc,
    thread::{self},
};

use crate::Persist;

pub mod account;
pub mod bank;
pub mod character;
pub mod error;
pub mod events;
pub mod grand_exchange;
pub mod items;
pub mod maps;
pub mod monsters;
pub mod npcs;
pub mod npcs_items;
pub mod resources;
pub mod server;
pub mod tasks;
pub mod tasks_rewards;

pub use crate::client::{
    account::AccountClient, bank::BankClient, character::CharacterClient, error::ClientError,
    events::EventsClient, grand_exchange::GrandExchangeClient, items::ItemsClient,
    maps::MapsClient, monsters::MonstersClient, npcs::NpcsClient, npcs_items::NpcsItemsClient,
    resources::ResourcesClient, server::ServerClient, tasks::TasksClient,
    tasks_rewards::TasksRewardsClient,
};

mod private {
    pub trait Sealed {}
}

/// Read-only access to an RCU (Read-Copy-Update) collection snapshot.
///
/// Each method takes an `Arc` snapshot of the inner `HashMap` (lock-free via
/// `ArcSwap`), then operates on it freely — callers never block concurrent
/// writes (e.g. background refresh).
pub trait CollectionClient: Data {
    fn get<Q>(&self, key: &Q) -> Option<Self::Entity>
    where
        Self::Key: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.data().get(key).cloned()
    }

    fn contains<Q>(&self, key: &Q) -> bool
    where
        Self::Key: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.data().contains_key(key)
    }

    fn len(&self) -> usize {
        self.data().len()
    }

    fn is_empty(&self) -> bool {
        self.data().is_empty()
    }

    fn any<F>(&self, f: F) -> bool
    where
        F: FnMut(&Self::Entity) -> bool,
    {
        self.data().values().any(f)
    }

    fn find<F>(&self, mut f: F) -> Option<Self::Entity>
    where
        F: FnMut(&Self::Entity) -> bool,
    {
        self.data().values().find(|v| f(v)).cloned()
    }

    fn min_by_key<F, R>(&self, mut f: F) -> Option<Self::Entity>
    where
        F: FnMut(&Self::Entity) -> R,
        R: Ord,
    {
        self.data().values().min_by_key(|v| f(v)).cloned()
    }

    fn max_by_key<F, R>(&self, mut f: F) -> Option<Self::Entity>
    where
        F: FnMut(&Self::Entity) -> R,
        R: Ord,
    {
        self.data().values().max_by_key(|v| f(v)).cloned()
    }

    fn iter(&self) -> CollectionIter<Self::Key, Self::Entity>
    where
        Self::Key: Clone + Hash + Eq,
    {
        let snapshot = self.data();
        let keys = snapshot.keys().cloned().collect();
        CollectionIter {
            snapshot,
            keys,
            pos: 0,
        }
    }
}

/// Interior-mutable collection backed by `ArcSwap<HashMap<K, V>>`.
///
/// Writers atomically swap the entire `HashMap` pointer (`ArcSwap::store`).
/// Readers take a lock-free `Arc` snapshot (`ArcSwap::load_full`).
pub trait Data: private::Sealed {
    type Entity: Clone;
    type Key: Hash + Eq;

    fn data(&self) -> Arc<HashMap<Self::Key, Self::Entity>>;
}

#[derive(Default, Debug, Clone, Deref)]
#[deref(forward)]
pub struct Client(Arc<ClientInner>);

#[derive(Default, Debug)]
pub struct ClientInner {
    pub account: AccountClient,
    pub server: ServerClient,
    pub events: EventsClient,
    pub resources: ResourcesClient,
    pub monsters: MonstersClient,
    pub items: ItemsClient,
    pub tasks: TasksClient,
    pub maps: MapsClient,
    pub npcs: NpcsClient,
    pub grand_exchange: GrandExchangeClient,
}

impl Client {
    #[must_use]
    pub fn new(url: String, token: String, account_name: String) -> Self {
        let api = ArtifactApi::new(url, token);
        let bank = BankClient::new(api.clone());
        let account = AccountClient::new(account_name, bank, api.clone());
        let server = ServerClient::new(api.clone());
        let events = EventsClient::new(api.clone());
        let resources = ResourcesClient::new(api.clone(), events.clone());
        let monsters = MonstersClient::new(api.clone(), events.clone());
        let tasks_rewards = TasksRewardsClient::new(api.clone());
        let tasks = TasksClient::new(api.clone(), tasks_rewards.clone());
        let npcs_items = NpcsItemsClient::new(api.clone());
        let npcs = NpcsClient::new(api.clone(), npcs_items);
        let items = ItemsClient::new(
            api.clone(),
            resources.clone(),
            monsters.clone(),
            tasks_rewards,
            npcs.clone(),
        );
        let maps = MapsClient::new(api.clone(), events.clone());
        let grand_exchange = GrandExchangeClient::new(api);
        Self(
            ClientInner {
                account,
                server,
                events,
                resources,
                monsters,
                items,
                tasks,
                maps,
                npcs,
                grand_exchange,
            }
            .into(),
        )
    }

    pub fn init(&self) {
        thread::scope(|s| {
            s.spawn(|| self.server.init());
            s.spawn(|| self.account.init());
            s.spawn(|| self.account.bank().init());
            s.spawn(|| {
                self.account.load_characters(
                    &self.items,
                    &self.resources,
                    &self.monsters,
                    &self.maps,
                    &self.npcs,
                    &self.tasks,
                    &self.server,
                    &self.grand_exchange,
                )
            });
            s.spawn(|| self.maps.init());
            s.spawn(|| self.items.init());
            s.spawn(|| self.resources.init());
            s.spawn(|| self.monsters.init());
            s.spawn(|| self.npcs.init());
            s.spawn(|| self.npcs.items().init());
            s.spawn(|| self.tasks.init());
            s.spawn(|| self.tasks.rewards().init());
            s.spawn(|| self.events.init());
        });
    }

    pub fn refresh_data(&self) {
        self.server.update_status();
        // self.account.refresh();
        // self.account.bank().refresh();
        // self.account.characters().refresh();
        // self.maps.refresh();
        self.items.refresh();
        self.resources.refresh();
        self.monsters.refresh();
        self.npcs.refresh();
        self.npcs.items().refresh();
        self.tasks.refresh();
        self.tasks.rewards().refresh();
        self.events.refresh();
    }
}

/// Lazily-yielding iterator over an RCU snapshot of a collection.
///
/// Clones keys upfront (cheap) and values on demand.
pub struct CollectionIter<K, V> {
    snapshot: Arc<HashMap<K, V>>,
    keys: Vec<K>,
    pos: usize,
}

impl<K: Clone + Hash + Eq, V: Clone> Iterator for CollectionIter<K, V> {
    type Item = V;

    fn next(&mut self) -> Option<V> {
        let key = &self.keys[self.pos];
        self.pos += 1;
        self.snapshot.get(key).cloned()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.keys.len() - self.pos;
        (remaining, Some(remaining))
    }
}

impl<K: Clone + Hash + Eq, V: Clone> ExactSizeIterator for CollectionIter<K, V> {}
