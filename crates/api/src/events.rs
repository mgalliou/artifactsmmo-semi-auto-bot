use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        events_api::{
            GetAllActiveEventsEventsActiveGetError, GetAllEventsEventsGetError,
            get_all_active_events_events_active_get, get_all_events_events_get,
        },
    },
    models::{
        ActiveEventSchema, EventSchema, StaticDataPageActiveEventSchema, StaticDataPageEventSchema,
    },
};
use std::{option::Option, sync::Arc};

#[derive(Default, Debug)]
pub struct EventsApi {
    configuration: Arc<Configuration>,
}

impl EventsApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn get_all(&self) -> Result<Vec<EventSchema>, Error<GetAllEventsEventsGetError>> {
        EventsRequest {
            configuration: &self.configuration,
        }
        .send()
    }

    pub fn get_active(
        &self,
    ) -> Result<Vec<ActiveEventSchema>, Error<GetAllActiveEventsEventsActiveGetError>> {
        ActiveEventsRequest {
            configuration: &self.configuration,
        }
        .send()
    }
}

struct EventsRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for EventsRequest<'a> {
    type Data = EventSchema;
    type Page = StaticDataPageEventSchema;
    type Error = GetAllEventsEventsGetError;

    fn request_page(&self, page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_events_events_get(self.configuration, None, Some(page), Some(100))
    }
}

impl DataPage<EventSchema> for StaticDataPageEventSchema {
    fn data(self) -> Vec<EventSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}

struct ActiveEventsRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for ActiveEventsRequest<'a> {
    type Data = ActiveEventSchema;
    type Page = StaticDataPageActiveEventSchema;
    type Error = GetAllActiveEventsEventsActiveGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_active_events_events_active_get(self.configuration, Some(current_page), Some(100))
    }
}

impl DataPage<ActiveEventSchema> for StaticDataPageActiveEventSchema {
    fn data(self) -> Vec<ActiveEventSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
