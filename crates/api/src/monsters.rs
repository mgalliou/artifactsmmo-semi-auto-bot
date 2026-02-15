use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        monsters_api::{GetAllMonstersMonstersGetError, get_all_monsters_monsters_get},
    },
    models::{DataPageMonsterSchema, MonsterSchema},
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct MonstersApi {
    configuration: Arc<Configuration>,
}

impl MonstersApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn get_all(&self) -> Result<Vec<MonsterSchema>, Error<GetAllMonstersMonstersGetError>> {
        MonstersRequest {
            configuration: &self.configuration,
        }
        .send()
    }
}

struct MonstersRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for MonstersRequest<'a> {
    type Data = MonsterSchema;
    type Page = DataPageMonsterSchema;
    type Error = GetAllMonstersMonstersGetError;

    fn request_page(&self, page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_monsters_monsters_get(
            self.configuration,
            None,
            None,
            None,
            None,
            Some(page),
            Some(100),
        )
    }
}

impl DataPage<MonsterSchema> for DataPageMonsterSchema {
    fn data(self) -> Vec<MonsterSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
