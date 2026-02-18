use crate::{
    ClientError, ItemsClient, MapsClient, MonstersClient, NpcsClient, ResourcesClient,
    ServerClient, TasksClient,
    character::{CharacterDataHandle, HasCharacterData},
    client::{bank::BankClient, character::CharacterClient},
    grand_exchange::GrandExchangeClient,
};
use api::ArtifactApi;
use itertools::Itertools;
use openapi::models::AccountAchievementSchema;
use std::sync::{Arc, RwLock};

pub trait Account {
    fn name(&self) -> &str;
    fn bank(&self) -> BankClient;
}

#[derive(Default, Debug, Clone)]
pub struct AccountClient(Arc<AccountInner>);

/// Hold and manage data related to a specific account
#[derive(Default, Debug)]
pub struct AccountInner {
    name: String,
    bank: BankClient,
    characters: RwLock<Vec<CharacterClient>>,
    achievements: RwLock<Vec<Arc<AccountAchievementSchema>>>,
    api: Arc<ArtifactApi>,
}

impl AccountClient {
    pub(crate) fn new(name: String, bank: BankClient, api: Arc<ArtifactApi>) -> Self {
        Self(Arc::new(AccountInner {
            bank,
            characters: Default::default(),
            achievements: RwLock::new(
                api.account
                    .achievements(&name)
                    .unwrap()
                    .into_iter()
                    .map(Arc::new)
                    .collect_vec(),
            ),
            name,
            api,
        }))
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn bank(&self) -> BankClient {
        self.0.bank.clone()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn load_characters(
        &self,
        items: Arc<ItemsClient>,
        resources: Arc<ResourcesClient>,
        monsters: Arc<MonstersClient>,
        maps: Arc<MapsClient>,
        npcs: Arc<NpcsClient>,
        tasks: Arc<TasksClient>,
        server: Arc<ServerClient>,
        grand_exchange: Arc<GrandExchangeClient>,
    ) -> Result<(), ClientError> {
        *self.0.characters.write().unwrap() = self
            .0
            .api
            .account
            .characters(self.name())
            .map_err(|e| ClientError::Api(Box::new(e)))?
            .data
            .into_iter()
            .enumerate()
            .map(|(id, data)| {
                CharacterClient::new(
                    id,
                    CharacterDataHandle::new(data),
                    self.clone(),
                    items.clone(),
                    resources.clone(),
                    monsters.clone(),
                    maps.clone(),
                    npcs.clone(),
                    tasks.clone(),
                    grand_exchange.clone(),
                    server.clone(),
                    self.0.api.clone(),
                )
            })
            .collect_vec();
        Ok(())
    }

    pub fn load_achievements(&self) -> Result<(), ClientError> {
        *self.0.achievements.write().unwrap() = self
            .0
            .api
            .account
            .achievements(self.name())
            .map_err(|e| ClientError::Api(Box::new(e)))?
            .into_iter()
            .map(Arc::new)
            .collect_vec();
        Ok(())
    }

    pub fn characters(&self) -> Vec<CharacterClient> {
        self.0.characters.read().unwrap().iter().cloned().collect()
    }

    pub fn get_character_by_name(&self, name: &str) -> Option<CharacterClient> {
        self.characters().iter().find(|c| c.name() == name).cloned()
    }

    pub fn achievements(&self) -> Vec<Arc<AccountAchievementSchema>> {
        self.0
            .achievements
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect_vec()
    }

    pub fn get_achievement(&self, code: &str) -> Option<Arc<AccountAchievementSchema>> {
        self.achievements().iter().find(|a| a.code == code).cloned()
    }
}
