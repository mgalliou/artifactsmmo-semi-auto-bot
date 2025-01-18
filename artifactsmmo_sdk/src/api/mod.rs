use artifactsmmo_openapi::apis::configuration::Configuration;
use std::sync::Arc;

pub use bank::BankApi;
pub use characters::CharactersApi;
pub use events::EventsApi;
pub use items::ItemsApi;
pub use maps::MapsApi;
pub use monsters::MonstersApi;
pub use my_characters::MyCharacterApi;
pub use resources::ResourcesApi;
pub use tasks::TasksApi;

pub mod bank;
pub mod characters;
pub mod events;
pub mod items;
pub mod maps;
pub mod monsters;
pub mod my_characters;
pub mod resources;
pub mod tasks;

pub struct ArtifactApi {
    pub bank: BankApi,
    pub character: CharactersApi,
    pub events: EventsApi,
    pub items: ItemsApi,
    pub maps: MapsApi,
    pub monsters: MonstersApi,
    pub my_character: MyCharacterApi,
    pub resources: ResourcesApi,
    pub tasks: TasksApi,
}

impl ArtifactApi {
    pub fn new(base_path: &str, token: &str) -> Self {
        let configuration = Arc::new({
            let mut configuration = Configuration::new();
            configuration.base_path = base_path.to_owned();
            configuration.bearer_access_token = Some(token.to_owned());
            configuration
        });
        Self {
            bank: BankApi::new(configuration.clone()),
            character: CharactersApi::new(configuration.clone()),
            events: EventsApi::new(configuration.clone()),
            items: ItemsApi::new(configuration.clone()),
            maps: MapsApi::new(configuration.clone()),
            monsters: MonstersApi::new(configuration.clone()),
            my_character: MyCharacterApi::new(configuration.clone()),
            resources: ResourcesApi::new(configuration.clone()),
            tasks: TasksApi::new(configuration.clone()),
        }
    }
}
