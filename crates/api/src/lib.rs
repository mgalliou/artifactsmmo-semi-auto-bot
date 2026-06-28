use derive_more::Deref;
use openapi::apis::{Error, configuration::Configuration};
use std::{
    sync::Arc,
    thread::{self},
};

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

#[derive(Default, Debug, Clone, Deref)]
#[deref(forward)]
pub struct ArtifactApi(Arc<ArtifactApiInner>);

#[derive(Default, Debug)]
pub struct ArtifactApiInner {
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
            let mut c = (*conf).clone();
            c.bearer_access_token = Some(token);
            c
        });
        Self(
            ArtifactApiInner {
                account: AccountApi::new(auth_conf.clone()),
                bank: BankApi::new(auth_conf.clone()),
                character: CharactersApi::new(conf.clone()),
                events: EventsApi::new(conf.clone()),
                grand_exchange: GrandExchangeApi::new(conf.clone()),
                items: ItemsApi::new(conf.clone()),
                maps: MapsApi::new(conf.clone()),
                monsters: MonstersApi::new(conf.clone()),
                my_character: MyCharacterApi::new(auth_conf),
                npcs: NpcsApi::new(conf.clone()),
                resources: ResourcesApi::new(conf.clone()),
                server: ServerApi::new(conf.clone()),
                tasks: TasksApi::new(conf),
            }
            .into(),
        )
    }
}

pub trait Paginate {
    type Data;
    type Page: DataPage<Self::Data>;
    type Error;

    fn send(&self) -> Result<Vec<Self::Data>, Error<Self::Error>>
    where
        Self: std::marker::Sync,
        <Self as Paginate>::Page: std::marker::Send,
        <Self as Paginate>::Error: std::marker::Send,
    {
        let mut data: Vec<Self::Data> = vec![];
        let response = self.request_page(1)?;
        let pages = response.pages();

        data.extend(response.data());
        if pages > 1 {
            thread::scope(|s| {
                let mut handles = vec![];
                for p in 2..pages {
                    handles.push(s.spawn(move || self.request_page(p)));
                }
                for h in handles {
                    let Ok(resp) = h.join().unwrap() else {
                        continue;
                    };
                    data.extend(resp.data());
                }
            });
        }
        Ok(data)
    }

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>>;
}

pub trait DataPage<T> {
    fn data(self) -> Vec<T>;
    fn pages(&self) -> u32;
}
