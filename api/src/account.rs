use artifactsmmo_openapi::{
    apis::{
        accounts_api::{
            get_account_characters_accounts_account_characters_get,
            GetAccountCharactersAccountsAccountCharactersGetError,
        },
        configuration::Configuration,
        Error,
    },
    models::CharactersListSchema,
};
use std::sync::Arc;

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
}
