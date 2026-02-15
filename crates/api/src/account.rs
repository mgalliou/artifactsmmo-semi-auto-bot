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
    },
    models::{AccountAchievementSchema, CharactersListSchema, DataPageAccountAchievementSchema},
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct AccountApi {
    configuration: Arc<Configuration>,
}

impl AccountApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn characters(
        &self,
        account: &str,
    ) -> Result<CharactersListSchema, Error<GetAccountCharactersAccountsAccountCharactersGetError>>
    {
        get_account_characters_accounts_account_characters_get(&self.configuration, account)
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
}

struct AchievementsRequest<'a> {
    configuration: &'a Configuration,
    account: &'a str,
}

impl<'a> Paginate for AchievementsRequest<'a> {
    type Data = AccountAchievementSchema;
    type Page = DataPageAccountAchievementSchema;
    type Error = GetAccountAchievementsAccountsAccountAchievementsGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_account_achievements_accounts_account_achievements_get(
            self.configuration,
            self.account,
            None,
            None,
            Some(current_page),
            Some(100),
        )
    }
}

impl DataPage<AccountAchievementSchema> for DataPageAccountAchievementSchema {
    fn data(self) -> Vec<AccountAchievementSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
