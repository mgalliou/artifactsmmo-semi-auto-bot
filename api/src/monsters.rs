use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        monsters_api::{
            get_all_monsters_monsters_get, get_monster_monsters_code_get,
            GetAllMonstersMonstersGetError, GetMonsterMonstersCodeGetError,
        },
        Error,
    },
    models::{MonsterResponseSchema, MonsterSchema},
};
use std::sync::Arc;

pub struct MonstersApi {
    configuration: Arc<Configuration>,
}

impl MonstersApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn all(
        &self,
        min_level: Option<i32>,
        max_level: Option<i32>,
        drop: Option<&str>,
    ) -> Result<Vec<MonsterSchema>, Error<GetAllMonstersMonstersGetError>> {
        let mut monsters: Vec<MonsterSchema> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = get_all_monsters_monsters_get(
                &self.configuration,
                min_level,
                max_level,
                drop,
                Some(current_page),
                Some(100),
            );
            match resp {
                Ok(resp) => {
                    monsters.extend(resp.data);
                    if let Some(Some(pages)) = resp.pages {
                        if current_page >= pages {
                            finished = true
                        }
                        current_page += 1;
                    } else {
                        // No pagination information, assume single page
                        finished = true
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(monsters)
    }

    pub fn info(
        &self,
        code: &str,
    ) -> Result<MonsterResponseSchema, Error<GetMonsterMonstersCodeGetError>> {
        get_monster_monsters_code_get(&self.configuration, code)
    }
}
