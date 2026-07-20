use crate::{
    ClientError, Code, ItemsClient, MapsClient, MonstersClient, NpcsClient, ResourcesClient,
    ServerClient, TasksClient,
    client::{
        bank::BankClient,
        character::{CharacterClient, CharacterRequestHandler},
    },
    entities::{AccountAchievement, Character, CharacterHandle, PendingItemHandle, RawPendingItem},
    grand_exchange::GrandExchangeClient,
};
use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use openapi::models::{AccountAchievementSchema, CharacterSchema, PendingItemSchema};
use std::sync::{Arc, RwLock};

pub(crate) type CharactersSource =
    Box<dyn Fn(&str) -> Result<Vec<CharacterSchema>, ClientError> + Send + Sync + 'static>;
pub(crate) type AccountAchievementsSource =
    Box<dyn Fn(&str) -> Result<Vec<AccountAchievementSchema>, ClientError> + Send + Sync + 'static>;
pub(crate) type PendingItemsSource =
    Box<dyn Fn() -> Result<Vec<PendingItemSchema>, ClientError> + Send + Sync + 'static>;
pub(crate) type CharacterHandlerBuilder = Box<
    dyn Fn(CharacterHandle, AccountClient, ServerClient) -> Arc<dyn CharacterRequestHandler>
        + Send
        + Sync
        + 'static,
>;

#[derive(Clone, Default, Deref)]
#[deref(forward)]
pub struct AccountClient(Arc<AccountClientInner>);

/// Hold and manage data related to a specific account
pub struct AccountClientInner {
    name: String,
    bank: BankClient,
    characters: RwLock<Vec<CharacterClient>>,
    achievements: RwLock<Vec<AccountAchievement>>,
    pending_items: ArcSwap<Vec<PendingItemHandle>>,
    fetch_characters: CharactersSource,
    fetch_achievements: AccountAchievementsSource,
    fetch_pending_items: PendingItemsSource,
    create_handler: CharacterHandlerBuilder,
}

impl Default for AccountClientInner {
    fn default() -> Self {
        Self {
            name: String::default(),
            bank: BankClient::default(),
            characters: RwLock::default(),
            achievements: RwLock::default(),
            pending_items: ArcSwap::default(),
            fetch_characters: Box::new(|_| panic!("AccountClient not initialized")),
            fetch_achievements: Box::new(|_| panic!("AccountClient not initialized")),
            fetch_pending_items: Box::new(|| panic!("AccountClient not initialized")),
            create_handler: Box::new(|_, _, _| panic!("AccountClient not initialized")),
        }
    }
}

impl AccountClient {
    #[must_use]
    pub(crate) fn new(
        name: String,
        bank: BankClient,
        fetch_characters: CharactersSource,
        fetch_achievements: AccountAchievementsSource,
        fetch_pending_items: PendingItemsSource,
        create_handler: CharacterHandlerBuilder,
    ) -> Self {
        Self(Arc::new(AccountClientInner {
            name,
            bank,
            characters: RwLock::default(),
            achievements: RwLock::default(),
            pending_items: ArcSwap::default(),
            fetch_characters,
            fetch_achievements,
            fetch_pending_items,
            create_handler,
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
        *self.characters.write().unwrap() = (self.fetch_characters)(self.name())?
            .into_iter()
            .enumerate()
            .map(|(id, schema)| {
                let data = CharacterHandle::new(schema);
                let handler = (self.create_handler)(data.clone(), self.clone(), server.clone());
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

    #[cfg(any(test, feature = "test-utils"))]
    pub fn add_character(&self, character: CharacterClient) {
        self.characters.write().unwrap().push(character);
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn add_achievement(&self, achievement: AccountAchievement) {
        self.achievements.write().unwrap().push(achievement);
    }

    fn load_achievements(&self) -> Result<(), ClientError> {
        *self.achievements.write().unwrap() = (self.fetch_achievements)(self.name())?
            .into_iter()
            .map(AccountAchievement::new)
            .collect_vec();
        Ok(())
    }

    fn load_pending_items(&self) -> Result<(), ClientError> {
        self.pending_items.store(Arc::new(
            (self.fetch_pending_items)()?
                .into_iter()
                .map(PendingItemHandle::new)
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
            .find(|i| i.load().id() == item.id)
        else {
            return;
        };
        pending.store(RawPendingItem::new(item));
    }
}
