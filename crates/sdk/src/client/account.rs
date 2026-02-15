use crate::{
    ClientError, ItemsClient, MapsClient, MonstersClient, NpcsClient, ResourcesClient,
    ServerClient, TasksClient,
    character::HasCharacterData,
    client::{bank::BankClient, character::CharacterClient},
    grand_exchange::GrandExchangeClient,
};
use api::ArtifactApi;
use openapi::models::AccountAchievementSchema;
use itertools::Itertools;
use std::sync::{Arc, RwLock};

#[derive(Default, Debug)]
pub struct AccountClient {
    pub name: String,
    pub bank: Arc<BankClient>,
    characters: RwLock<Vec<Arc<CharacterClient>>>,
    achievements: RwLock<Vec<Arc<AccountAchievementSchema>>>,
    api: Arc<ArtifactApi>,
}

impl AccountClient {
    pub(crate) fn new(name: String, bank: Arc<BankClient>, api: Arc<ArtifactApi>) -> Self {
        Self {
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
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn load_characters(
        &self,
        account: Arc<AccountClient>,
        items: Arc<ItemsClient>,
        resources: Arc<ResourcesClient>,
        monsters: Arc<MonstersClient>,
        maps: Arc<MapsClient>,
        npcs: Arc<NpcsClient>,
        tasks: Arc<TasksClient>,
        server: Arc<ServerClient>,
        grand_exchange: Arc<GrandExchangeClient>,
    ) -> Result<(), ClientError> {
        *self.characters.write().unwrap() = self
            .api
            .account
            .characters(&account.name)
            .map_err(|e| ClientError::Api(Box::new(e)))?
            .data
            .into_iter()
            .enumerate()
            .map(|(id, data)| {
                CharacterClient::new(
                    id,
                    Arc::new(RwLock::new(Arc::new(data))),
                    account.clone(),
                    items.clone(),
                    resources.clone(),
                    monsters.clone(),
                    maps.clone(),
                    npcs.clone(),
                    tasks.clone(),
                    grand_exchange.clone(),
                    server.clone(),
                    self.api.clone(),
                )
            })
            .map(Arc::new)
            .collect_vec();
        Ok(())
    }

    pub fn load_achievements(&self) -> Result<(), ClientError> {
        *self.achievements.write().unwrap() = self
            .api
            .account
            .achievements(&self.name)
            .map_err(|e| ClientError::Api(Box::new(e)))?
            .into_iter()
            .map(Arc::new)
            .collect_vec();
        Ok(())
    }

    pub fn characters(&self) -> Vec<Arc<CharacterClient>> {
        self.characters.read().unwrap().iter().cloned().collect()
    }

    pub fn get_character_by_name(&self, name: &str) -> Option<Arc<CharacterClient>> {
        self.characters
            .read()
            .unwrap()
            .iter()
            .find(|c| c.name() == name)
            .cloned()
    }

    pub fn achievements(&self) -> Vec<Arc<AccountAchievementSchema>> {
        self.achievements
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
