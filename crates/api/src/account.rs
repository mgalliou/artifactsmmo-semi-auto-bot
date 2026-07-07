use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        accounts_api::{
            GetAccountAchievementsAccountsAccountAchievementsGetError,
            GetAccountCharactersAccountsAccountCharactersGetError,
            get_account_achievements_accounts_account_achievements_get,
            get_account_characters_accounts_account_characters_get,
        },
        configuration::Configuration,
        my_account_api::{
            GetPendingItemsMyPendingItemsGetError, get_pending_items_my_pending_items_get,
        },
    },
    models::{
        AccountAchievementSchema, CharactersListSchema, DataPageAccountAchievementSchema,
        DataPagePendingItemSchema, PendingItemSchema,
    },
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct AccountApi {
    configuration: Arc<Configuration>,
}

impl AccountApi {
    pub(crate) const fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn characters(
        &self,
        account: &str,
    ) -> Result<CharactersListSchema, Error<GetAccountCharactersAccountsAccountCharactersGetError>>
    {
        crate::runtime().block_on(get_account_characters_accounts_account_characters_get(
            &self.configuration,
            account,
        ))
    }

    pub fn achievements(
        &self,
        account: &str,
    ) -> Result<
        Vec<AccountAchievementSchema>,
        Error<GetAccountAchievementsAccountsAccountAchievementsGetError>,
    > {
        AchievementsRequest {
            configuration: &self.configuration,
            account,
        }
        .send()
    }

    pub fn pending_items(
        &self,
    ) -> Result<Vec<PendingItemSchema>, Error<GetPendingItemsMyPendingItemsGetError>> {
        PendingItemsRequest {
            configuration: &self.configuration,
        }
        .send()
    }
}

struct AchievementsRequest<'a> {
    configuration: &'a Configuration,
    account: &'a str,
}

impl Paginate for AchievementsRequest<'_> {
    type Data = AccountAchievementSchema;
    type Page = DataPageAccountAchievementSchema;
    type Error = GetAccountAchievementsAccountsAccountAchievementsGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        crate::runtime().block_on(get_account_achievements_accounts_account_achievements_get(
            self.configuration,
            self.account,
            None,
            None,
            Some(current_page),
            Some(100),
        ))
    }
}

impl DataPage<AccountAchievementSchema> for DataPageAccountAchievementSchema {
    fn data(self) -> Vec<AccountAchievementSchema> {
        self.data
    }

    fn pages(&self) -> u32 {
        self.pages
    }
}

struct PendingItemsRequest<'a> {
    configuration: &'a Configuration,
}

impl Paginate for PendingItemsRequest<'_> {
    type Data = PendingItemSchema;
    type Page = DataPagePendingItemSchema;
    type Error = GetPendingItemsMyPendingItemsGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        crate::runtime().block_on(get_pending_items_my_pending_items_get(
            self.configuration,
            Some(current_page),
            Some(100),
        ))
    }
}

impl DataPage<PendingItemSchema> for DataPagePendingItemSchema {
    fn data(self) -> Vec<PendingItemSchema> {
        self.data
    }

    fn pages(&self) -> u32 {
        self.pages
    }
}
