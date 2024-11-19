use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        events_api::{
            get_all_active_events_events_active_get, get_all_events_events_get, GetAllActiveEventsEventsActiveGetError, GetAllEventsEventsGetError
        },
        Error,
    },
    models::{ActiveEventSchema, EventSchema},
};

#[derive(Default)]
pub struct EventsApi {
    pub configuration: Configuration,
}

impl EventsApi {
    pub fn new(base_path: &str) -> Self {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        EventsApi { configuration }
    }

    pub fn all(
        &self,
    ) -> Result<Vec<EventSchema>, Error<GetAllEventsEventsGetError>> {
        let mut events: Vec<EventSchema> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = get_all_events_events_get(
                &self.configuration,
                Some(current_page),
                Some(100),
            );
            match resp {
                Ok(resp) => {
                    events.extend(resp.data);
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
        Ok(events)
    }

    pub fn active(
        &self,
    ) -> Result<Vec<ActiveEventSchema>, Error<GetAllActiveEventsEventsActiveGetError>> {
        let mut events: Vec<ActiveEventSchema> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = get_all_active_events_events_active_get(
                &self.configuration,
                Some(current_page),
                Some(100),
            );
            match resp {
                Ok(resp) => {
                    events.extend(resp.data);
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
        Ok(events)
    }
}
