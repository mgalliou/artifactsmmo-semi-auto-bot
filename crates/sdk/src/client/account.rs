use crate::{
    ClientError, Code, ItemsClient, MapsClient, MonstersClient, NpcsClient, ResourcesClient,
    ServerClient, TasksClient,
    client::{
        bank::BankClient,
        character::{
            CharacterClient, CharacterRequestHandler, request_handler::CharacterHttpRequestHandler,
        },
    },
    entities::{AccountAchievement, Character, CharacterHandle, PendingItemHandle, RawPendingItem},
    grand_exchange::GrandExchangeClient,
};
use ::api::ArtifactApi;
use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use openapi::models::PendingItemSchema;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default, Deref)]
#[deref(forward)]
pub struct AccountClient(Arc<AccountClientInner>);

/// Hold and manage data related to a specific account
#[derive(Default)]
pub struct AccountClientInner {
    api: ArtifactApi,
    name: String,
    bank: BankClient,
    characters: RwLock<Vec<CharacterClient>>,
    achievements: RwLock<Vec<AccountAchievement>>,
    pending_items: ArcSwap<Vec<PendingItemHandle>>,
}

impl AccountClient {
    pub(crate) fn new(name: String, bank: BankClient, api: ArtifactApi) -> Self {
        Self(Arc::new(AccountClientInner {
            api,
            name,
            bank,
            characters: RwLock::default(),
            achievements: RwLock::default(),
            pending_items: ArcSwap::default(),
        }))
    }

    pub fn init(&self) {
        let _ = self.load_achievements();
        let _ = self.load_pending_items();
        info!("Account achievements initilized");
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn bank(&self) -> BankClient {
        self.bank.clone()
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
        *self.characters.write().unwrap() = self
            .api
            .account
            .characters(self.name())
            .map_err(|e| ClientError::Api(Box::new(e)))?
            .data
            .into_iter()
            .enumerate()
            .map(|(id, data)| {
                let data: CharacterHandle = data.into();
                let handler: Arc<dyn CharacterRequestHandler> =
                    Arc::new(CharacterHttpRequestHandler::new(
                        self.api.clone(),
                        data.clone(),
                        self.clone(),
                        server.clone(),
                    ));
                CharacterClient::new(
                    id,
                    data,
                    handler,
                    self.clone(),
                    items.clone(),
                    resources.clone(),
                    monsters.clone(),
                    maps.clone(),
                    npcs.clone(),
                    tasks.clone(),
                    grand_exchange.clone(),
                )
            })
            .collect_vec();
        info!("Account character loaded");
        Ok(())
    }

    pub fn load_achievements(&self) -> Result<(), ClientError> {
        *self.achievements.write().unwrap() = self
            .api
            .account
            .achievements(self.name())
            .map_err(|e| ClientError::Api(Box::new(e)))?
            .into_iter()
            .map(AccountAchievement::new)
            .collect_vec();
        Ok(())
    }

    pub fn load_pending_items(&self) -> Result<(), ClientError> {
        self.pending_items.store(Arc::new(
            self.api
                .account
                .pending_items()
                .map_err(|e| ClientError::Api(Box::new(e)))?
                .into_iter()
                .map(PendingItemHandle::from)
                .collect_vec(),
        ));
        Ok(())
    }

    #[must_use]
    pub fn characters(&self) -> Vec<CharacterClient> {
        self.characters.read().unwrap().iter().cloned().collect()
    }

    #[must_use]
    pub fn get_character(&self, name: &str) -> Option<CharacterClient> {
        self.characters()
            .iter()
            .find(|c| *c.name() == *name)
            .cloned()
    }

    #[must_use]
    pub fn achievements(&self) -> Vec<AccountAchievement> {
        self.achievements
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect_vec()
    }

    #[must_use]
    pub fn get_achievement(&self, code: &str) -> Option<AccountAchievement> {
        self.achievements()
            .iter()
            .find(|a| a.code() == code)
            .cloned()
    }

    #[must_use]
    pub fn pending_items(&self) -> Vec<PendingItemHandle> {
        self.pending_items.load().iter().cloned().collect_vec()
    }

    pub fn update_pending_item(&self, item: PendingItemSchema) {
        let Some(pending) = self
            .pending_items()
            .into_iter()
            .find(|i| *i.load().id() == item.id)
        else {
            return;
        };
        pending.store(RawPendingItem::new(item));
    }
}
