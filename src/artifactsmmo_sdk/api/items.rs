use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        items_api::{get_all_items_items_get, get_item_items_code_get, GetItemItemsCodeGetError},
        Error,
    },
    models::ItemResponseSchema,
};

pub struct ItemsApi {
    pub configuration: Configuration,
}

impl ItemsApi {
    pub fn new(base_path: &str, token: &str) -> ItemsApi {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        ItemsApi { configuration }
    }

    pub fn all(&self) -> Result<artifactsmmo_openapi::models::DataPageItemSchema, Error<artifactsmmo_openapi::apis::items_api::GetAllItemsItemsGetError>> {
        get_all_items_items_get(
            &self.configuration,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn info(&self, code: &str) -> Result<ItemResponseSchema, Error<GetItemItemsCodeGetError>> {
        get_item_items_code_get(&self.configuration, code)
    }
}
