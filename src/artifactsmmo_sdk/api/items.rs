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

    pub fn all(
        &self,
        min_level: Option<i32>,
        max_level: Option<i32>,
        name: Option<&str>,
        r#type: Option<&str>,
        craft_skill: Option<&str>,
        craft_material: Option<&str>,
        page: Option<i32>,
        size: Option<i32>,
    ) -> Result<
        artifactsmmo_openapi::models::DataPageItemSchema,
        Error<artifactsmmo_openapi::apis::items_api::GetAllItemsItemsGetError>,
    > {
        get_all_items_items_get(
            &self.configuration,
            min_level,
            max_level,
            name,
            r#type,
            craft_skill,
            craft_material,
            page,
            size,
        )
    }

    pub fn info(&self, code: &str) -> Result<ItemResponseSchema, Error<GetItemItemsCodeGetError>> {
        get_item_items_code_get(&self.configuration, code)
    }
}
