use crate::Persist;
use api::ArtifactApi;
use derive_more::Deref;
use itertools::Itertools;
use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::Arc,
    thread::{self},
};

pub use crate::client::{
    account::AccountClient, bank::BankClient, character::CharacterClient, error::ClientError,
    events::EventsClient, grand_exchange::GrandExchangeClient, items::ItemsClient,
    maps::MapsClient, monsters::MonstersClient, npcs::NpcsClient, npcs_items::NpcsItemsClient,
    resources::ResourcesClient, server::ServerClient, tasks::TasksClient,
    tasks_rewards::TasksRewardsClient,
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

pub mod private {
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

    fn all(&self) -> Vec<Self::Entity> {
        self.data().values().cloned().collect_vec()
    }

    fn filtered<F>(&self, mut f: F) -> Vec<Self::Entity>
    where
        F: FnMut(&Self::Entity) -> bool,
    {
        self.data()
            .values()
            .filter(|v| f(*v))
            .cloned()
            .collect_vec()
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
