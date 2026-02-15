use openapi::apis::{Error, configuration::Configuration};
use std::sync::Arc;

pub use account::AccountApi;
pub use bank::BankApi;
pub use characters::CharactersApi;
pub use events::EventsApi;
pub use grand_exchange::GrandExchangeApi;
pub use items::ItemsApi;
pub use maps::MapsApi;
pub use monsters::MonstersApi;
pub use my_characters::MyCharacterApi;
pub use npcs::NpcsApi;
pub use resources::ResourcesApi;
pub use server::ServerApi;
pub use tasks::TasksApi;

pub mod account;
pub mod bank;
pub mod characters;
pub mod events;
pub mod grand_exchange;
pub mod items;
pub mod maps;
pub mod monsters;
pub mod my_characters;
pub mod npcs;
pub mod resources;
pub mod server;
pub mod tasks;

#[derive(Default, Debug)]
pub struct ArtifactApi {
    pub account: AccountApi,
    pub bank: BankApi,
    pub character: CharactersApi,
    pub events: EventsApi,
    pub grand_exchange: GrandExchangeApi,
    pub items: ItemsApi,
    pub maps: MapsApi,
    pub monsters: MonstersApi,
    pub my_character: MyCharacterApi,
    pub npcs: NpcsApi,
    pub resources: ResourcesApi,
    pub server: ServerApi,
    pub tasks: TasksApi,
}

impl ArtifactApi {
    pub fn new(base_path: String, token: String) -> Self {
        let conf = Arc::new({
            let mut c = Configuration::new();
            c.base_path = base_path;
            c
        });
        let auth_conf = Arc::new({
            let mut c = (*conf.clone()).clone();
            c.bearer_access_token = Some(token);
            c
        });
        Self {
            account: AccountApi::new(auth_conf.clone()),
            bank: BankApi::new(auth_conf.clone()),
            character: CharactersApi::new(conf.clone()),
            events: EventsApi::new(conf.clone()),
            grand_exchange: GrandExchangeApi::new(conf.clone()),
            items: ItemsApi::new(conf.clone()),
            maps: MapsApi::new(conf.clone()),
            monsters: MonstersApi::new(conf.clone()),
            my_character: MyCharacterApi::new(auth_conf.clone()),
            npcs: NpcsApi::new(conf.clone()),
            resources: ResourcesApi::new(conf.clone()),
            server: ServerApi::new(conf.clone()),
            tasks: TasksApi::new(conf.clone()),
        }
    }
}

pub trait Paginate {
    type Data;
    type Page: DataPage<Self::Data>;
    type Error;

    fn send(&self) -> Result<Vec<Self::Data>, Error<Self::Error>> {
        let mut npcs: Vec<Self::Data> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = self.request_page(current_page)?;
            if let Some(pages) = resp.pages() {
                if current_page >= pages {
                    finished = true
                }
                current_page += 1;
            } else {
                // No pagination information, assume single page
                finished = true
            }
            npcs.extend(resp.data());
        }
        Ok(npcs)
    }
    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>>;
}

pub trait DataPage<T> {
    fn data(self) -> Vec<T>;
    fn pages(&self) -> Option<u32>;
}
