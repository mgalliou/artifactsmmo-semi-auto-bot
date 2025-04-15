use artifactsmmo_openapi::apis::configuration::Configuration;
use std::sync::Arc;

pub use account::AccountApi;
pub use bank::BankApi;
pub use characters::CharactersApi;
pub use events::EventsApi;
pub use items::ItemsApi;
pub use maps::MapsApi;
pub use monsters::MonstersApi;
pub use my_characters::MyCharacterApi;
pub use resources::ResourcesApi;
pub use server::ServerApi;
pub use tasks::TasksApi;

pub mod account;
pub mod bank;
pub mod characters;
pub mod events;
pub mod items;
pub mod maps;
pub mod monsters;
pub mod my_characters;
pub mod resources;
pub mod server;
pub mod tasks;

pub struct ArtifactApi {
    pub account: AccountApi,
    pub bank: BankApi,
    pub character: CharactersApi,
    pub events: EventsApi,
    pub items: ItemsApi,
    pub maps: MapsApi,
    pub monsters: MonstersApi,
    pub my_character: MyCharacterApi,
    pub resources: ResourcesApi,
    pub tasks: TasksApi,
    pub server: ServerApi,
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
            items: ItemsApi::new(conf.clone()),
            maps: MapsApi::new(conf.clone()),
            monsters: MonstersApi::new(conf.clone()),
            my_character: MyCharacterApi::new(auth_conf.clone()),
            resources: ResourcesApi::new(conf.clone()),
            tasks: TasksApi::new(conf.clone()),
            server: ServerApi::new(conf.clone()),
        }
    }
}
