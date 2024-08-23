use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        monsters_api::{
            get_all_monsters_monsters_get, get_monster_monsters_code_get,
            GetAllMonstersMonstersGetError, GetMonsterMonstersCodeGetError,
        },
        Error,
    },
    models::{DataPageMonsterSchema, MonsterResponseSchema},
};

pub struct MonstersApi {
    configuration: Configuration,
}

impl MonstersApi {
    pub fn new(base_path: &str, token: &str) -> MonstersApi {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        MonstersApi { configuration }
    }

    pub fn all(
        &self,
        min_level: Option<i32>,
        max_level: Option<i32>,
        drop: Option<&str>,
        page: Option<i32>,
        size: Option<i32>,
    ) -> Result<DataPageMonsterSchema, Error<GetAllMonstersMonstersGetError>> {
        get_all_monsters_monsters_get(&self.configuration, min_level, max_level, drop, page, size)
    }

    pub fn info(
        &self,
        code: &str,
    ) -> Result<MonsterResponseSchema, Error<GetMonsterMonstersCodeGetError>> {
        get_monster_monsters_code_get(&self.configuration, code)
    }
}
