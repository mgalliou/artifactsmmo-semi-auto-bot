use api::ArtifactApi;
use std::{
    ops::Deref,
    sync::Arc,
    thread::{self},
};

use crate::Persist;
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

#[derive(Default, Debug, Clone)]
pub struct Client(Arc<ClientInner>);

impl Deref for Client {
    type Target = ClientInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    pub fn new(url: String, account_name: String, token: String) -> Result<Self, ClientError> {
        let api = ArtifactApi::new(url, token);
        let (bank, events, server, tasks, npcs) = thread::scope(|s| {
            let bank_handle = s.spawn(|| BankClient::new(&api));
            let events_handle = s.spawn(|| EventsClient::new(api.clone()));
            let server_handle = s.spawn(|| ServerClient::new(api.clone()));
            let tasks_handle =
                s.spawn(|| TasksClient::new(api.clone(), TasksRewardsClient::new(api.clone())));

            let npcs_handle =
                s.spawn(|| NpcsClient::new(api.clone(), NpcsItemsClient::new(api.clone())));

            (
                bank_handle.join().unwrap(),
                events_handle.join().unwrap(),
                server_handle.join().unwrap(),
                tasks_handle.join().unwrap(),
                npcs_handle.join().unwrap(),
            )
        });
        let (resources, monsters, maps) = thread::scope(|s| {
            let resources_handle = s.spawn(|| ResourcesClient::new(api.clone(), events.clone()));

            let monsters_handle = s.spawn(|| MonstersClient::new(api.clone(), events.clone()));

            let maps_handle = s.spawn(|| MapsClient::new(&api.clone(), events.clone()));

            (
                resources_handle.join().unwrap(),
                monsters_handle.join().unwrap(),
                maps_handle.join().unwrap(),
            )
        });

        let items = ItemsClient::new(
            api.clone(),
            resources.clone(),
            monsters.clone(),
            tasks.rewards(),
            npcs.clone(),
        );

        let account = AccountClient::new(account_name, bank, &api);
        let grand_exchange = GrandExchangeClient::new(api);
        account.load_characters(
            &items,
            &resources,
            &monsters,
            &maps,
            &npcs,
            &tasks,
            &server,
            &grand_exchange,
        )?;

        Ok(Self(
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
        ))
    }

    pub fn refresh_data(&self) {
        self.items.refresh();
        self.monsters.refresh();
        self.events.refresh();
        // TODO: implement this
        //self.maps.refresh();
        self.tasks.refresh();
        self.tasks.rewards().refresh();
        self.npcs.refresh();
        self.npcs.items().refresh();
        self.server.update_status();
    }
}
