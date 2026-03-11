use crate::{
    ClientError, Code, ItemsClient, MapsClient, MonstersClient, NpcsClient, ResourcesClient,
    ServerClient, TasksClient,
    character::data_handle::CharacterDataHandle,
    client::{bank::BankClient, character::CharacterClient},
    entities::{AccountAchievement, Character},
    grand_exchange::GrandExchangeClient,
};
use api::ArtifactApi;
use itertools::Itertools;
use std::sync::{Arc, RwLock};

#[derive(Default, Debug, Clone)]
pub struct AccountClient(Arc<AccountClientInner>);

/// Hold and manage data related to a specific account
#[derive(Default, Debug)]
struct AccountClientInner {
    api: ArtifactApi,
    name: String,
    bank: BankClient,
    characters: RwLock<Vec<CharacterClient>>,
    achievements: RwLock<Vec<AccountAchievement>>,
}

impl AccountClient {
    pub(crate) fn new(name: String, bank: BankClient, api: &ArtifactApi) -> Self {
        Self(
            AccountClientInner {
                api: api.clone(),
                bank,
                characters: RwLock::default(),
                achievements: RwLock::new(
                    api.account
                        .achievements(&name)
                        .unwrap()
                        .into_iter()
                        .map(AccountAchievement::new)
                        .collect_vec(),
                ),
                name,
            }
            .into(),
        )
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
        items: &ItemsClient,
        resources: &ResourcesClient,
        monsters: &MonstersClient,
        maps: &MapsClient,
        npcs: &NpcsClient,
        tasks: &TasksClient,
        server: &ServerClient,
        grand_exchange: &GrandExchangeClient,
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
            .map(AccountAchievement::new)
            .collect_vec();
        Ok(())
    }

    pub fn characters(&self) -> Vec<CharacterClient> {
        self.0.characters.read().unwrap().iter().cloned().collect()
    }

    pub fn get_character(&self, name: &str) -> Option<CharacterClient> {
        self.characters().iter().find(|c| c.name() == name).cloned()
    }

    pub fn achievements(&self) -> Vec<AccountAchievement> {
        self.0
            .achievements
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect_vec()
    }

    pub fn get_achievement(&self, code: &str) -> Option<AccountAchievement> {
        self.achievements()
            .iter()
            .find(|a| a.code() == code)
            .cloned()
    }
}
