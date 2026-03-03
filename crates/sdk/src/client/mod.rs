use api::ArtifactApi;
use std::{sync::Arc, thread};

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

#[derive(Default, Debug)]
pub struct Client {
    pub account: AccountClient,
    pub server: Arc<ServerClient>,
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
        let api = Arc::new(ArtifactApi::new(url, token));

        let (bank_res, events, server, tasks, npcs) = thread::scope(|s| {
            let api_clone = api.clone();
            let bank_handle = s.spawn(move || {
                let bank_details = api_clone
                    .bank
                    .get_details()
                    .map_err(|e| ClientError::Api(Box::new(e)))?;
                let bank_items = api_clone
                    .bank
                    .get_items()
                    .map_err(|e| ClientError::Api(Box::new(e)))?;
                Ok(BankClient::new(*bank_details.data, bank_items))
            });

            let api_clone = api.clone();
            let events_handle = s.spawn(move || EventsClient::new(api_clone.clone()));

            let api_clone = api.clone();
            let server_handle = s.spawn(move || Arc::new(ServerClient::new(api_clone.clone())));

            let api_clone = api.clone();
            let tasks_handle = s.spawn(move || {
                TasksClient::new(
                    api_clone.clone(),
                    TasksRewardsClient::new(api_clone.clone()),
                )
            });

            let api_clone = api.clone();
            let npcs_handle = s.spawn(move || {
                NpcsClient::new(api_clone.clone(), NpcsItemsClient::new(api_clone.clone()))
            });

            (
                bank_handle.join().unwrap(),
                events_handle.join().unwrap(),
                server_handle.join().unwrap(),
                tasks_handle.join().unwrap(),
                npcs_handle.join().unwrap(),
            )
        });

        let bank: BankClient = bank_res?;

        let (resources, monsters, maps) = thread::scope(|s| {
            let api_clone = api.clone();
            let events_clone = events.clone();
            let resources_handle =
                s.spawn(move || ResourcesClient::new(api_clone.clone(), events_clone));

            let api_clone = api.clone();
            let events_clone = events.clone();
            let monsters_handle =
                s.spawn(move || MonstersClient::new(api_clone.clone(), events_clone));

            let api_clone = api.clone();
            let events_clone = events.clone();
            let maps_handle = s.spawn(move || MapsClient::new(&api_clone, events_clone));

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
            tasks.rewards().clone(),
            npcs.clone(),
        );

        let account = AccountClient::new(account_name, bank, api.clone());
        let grand_exchange = GrandExchangeClient::new(api.clone());
        account.load_characters(
            items.clone(),
            resources.clone(),
            monsters.clone(),
            maps.clone(),
            npcs.clone(),
            tasks.clone(),
            server.clone(),
            grand_exchange.clone(),
        )?;

        Ok(Self {
            account,
            items,
            monsters,
            resources,
            server,
            events,
            tasks,
            maps,
            npcs,
            grand_exchange,
        })
    }

    pub fn refresh_data(&self) {
        self.items.refresh();
        todo!()
    }
}
