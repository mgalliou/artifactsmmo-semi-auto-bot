use crate::{
    Cached,
    entities::{Event, Item, Monster, Npc, Resource, Task, TaskReward},
};
use api::ArtifactApi;
use derive_more::Deref;
use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::Arc,
    thread::{self},
};

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

/// Read-only access to an Read-Copy-Update collection snapshot.
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

#[derive(Clone, Deref)]
#[deref(forward)]
pub struct Client(Arc<ClientInner>);

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

        let events = {
            let api_fetch = api.clone();
            EventsClient::new(
                ".cache/events.json",
                Box::new(move || {
                    api_fetch
                        .events
                        .get_all()
                        .unwrap()
                        .into_iter()
                        .map(|event| (event.code.clone(), Event::new(event)))
                        .collect()
                }),
                api.clone(),
            )
        };

        let resources = {
            let api = api.clone();
            ResourcesClient::new(
                ".cache/resources.json",
                Box::new(move || {
                    api.resources
                        .get_all()
                        .unwrap()
                        .into_iter()
                        .map(|r| (r.code.clone(), Resource::new(r)))
                        .collect()
                }),
                events.clone(),
            )
        };

        let monsters = {
            let api = api.clone();
            MonstersClient::new(
                ".cache/monsters.json",
                Box::new(move || {
                    api.monsters
                        .get_all()
                        .unwrap()
                        .into_iter()
                        .map(|m| (m.code.clone(), Monster::new(m)))
                        .collect()
                }),
                events.clone(),
            )
        };

        let tasks_rewards = {
            let api = api.clone();
            TasksRewardsClient::new(
                ".cache/tasks_rewards.json",
                Box::new(move || {
                    api.tasks
                        .get_rewards()
                        .unwrap()
                        .into_iter()
                        .map(|tr| (tr.code.clone(), TaskReward::new(tr)))
                        .collect()
                }),
            )
        };

        let tasks = {
            let api = api.clone();
            TasksClient::new(
                ".cache/tasks.json",
                Box::new(move || {
                    api.tasks
                        .get_all()
                        .unwrap()
                        .into_iter()
                        .map(|task| (task.code.clone(), Task::new(task)))
                        .collect()
                }),
                tasks_rewards.clone(),
            )
        };

        let npcs_items = NpcsItemsClient::new(
            ".cache/npcs_items.json",
            Box::new({
                let api = api.clone();
                move || {
                    api.npcs
                        .get_items()
                        .unwrap()
                        .into_iter()
                        .map(|npc| (npc.code.clone(), crate::entities::NpcItem::new(npc)))
                        .collect()
                }
            }),
        );

        let npcs = {
            let api = api.clone();
            NpcsClient::new(
                ".cache/npcs.json",
                Box::new(move || {
                    api.npcs
                        .get_all()
                        .unwrap()
                        .into_iter()
                        .map(|npc| (npc.code.clone(), Npc::new(npc)))
                        .collect()
                }),
                npcs_items,
            )
        };

        let items = {
            let api = api.clone();
            ItemsClient::new(
                ".cache/items.json",
                Box::new(move || {
                    api.items
                        .get_all()
                        .unwrap()
                        .into_iter()
                        .map(|i| (i.code.clone(), Item::new(i)))
                        .collect()
                }),
                resources.clone(),
                monsters.clone(),
                tasks_rewards,
                npcs.clone(),
            )
        };

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
