use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        tasks_api::{
            GetAllTasksRewardsTasksRewardsGetError, GetAllTasksTasksListGetError,
            get_all_tasks_rewards_tasks_rewards_get, get_all_tasks_tasks_list_get,
        },
    },
    models::{DataPageDropRateSchema, DataPageTaskFullSchema, DropRateSchema, TaskFullSchema},
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct TasksApi {
    configuration: Arc<Configuration>,
}

impl TasksApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn get_all(&self) -> Result<Vec<TaskFullSchema>, Error<GetAllTasksTasksListGetError>> {
        TasksRequest {
            configuration: &self.configuration,
        }
        .send()
    }

    pub fn get_rewards(
        &self,
    ) -> Result<Vec<DropRateSchema>, Error<GetAllTasksRewardsTasksRewardsGetError>> {
        TasksRewardsRequest {
            configuration: &self.configuration,
        }
        .send()
    }
}

struct TasksRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for TasksRequest<'a> {
    type Data = TaskFullSchema;
    type Page = DataPageTaskFullSchema;
    type Error = GetAllTasksTasksListGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_tasks_tasks_list_get(
            self.configuration,
            None,
            None,
            None,
            None,
            Some(current_page),
            Some(100),
        )
    }
}

impl DataPage<TaskFullSchema> for DataPageTaskFullSchema {
    fn data(self) -> Vec<TaskFullSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}

struct TasksRewardsRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for TasksRewardsRequest<'a> {
    type Data = DropRateSchema;
    type Page = DataPageDropRateSchema;
    type Error = GetAllTasksRewardsTasksRewardsGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_tasks_rewards_tasks_rewards_get(self.configuration, Some(current_page), Some(100))
    }
}

impl DataPage<DropRateSchema> for DataPageDropRateSchema {
    fn data(self) -> Vec<DropRateSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
