use super::{api::events::EventsApi, config::Config, ActiveEventSchemaExt, MapSchemaExt};
use artifactsmmo_openapi::models::ActiveEventSchema;
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use log::info;
use std::sync::RwLock;

pub struct Events {
    api: EventsApi,
    pub events: RwLock<Vec<ActiveEventSchema>>,
    last_refresh: RwLock<DateTime<Utc>>,
}

impl Events {
    pub fn new(config: &Config) -> Self {
        let events = Self {
            api: EventsApi::new(&config.base_url, &config.token),
            events: RwLock::new(vec![]),
            last_refresh: RwLock::new(DateTime::<Utc>::MIN_UTC),
        };
        events.refresh();
        events
    }

    pub fn refresh(&self) {
        let now = Utc::now();
        if Utc::now() - *self.last_refresh.read().unwrap() > Duration::seconds(30) {
            if let Ok(mut events) = self.events.write() {
                self.last_refresh.write().unwrap().clone_from(&now);
                if let Ok(new) = self.api.all() {
                    *events = new;
                    info!("events refreshed");
                }
            }
        }
    }

    pub fn of_type(&self, r#type: &str) -> Vec<ActiveEventSchema> {
        self.events
            .read()
            .iter()
            .flat_map(|e| {
                e.iter()
                    .filter(|e| e.map.content().is_some_and(|c| c.r#type == r#type))
                    .cloned()
            })
            .collect_vec()
    }
}

impl ActiveEventSchemaExt for ActiveEventSchema {
    fn content_code(&self) -> &String {
        self.map.content.as_ref().map(|c| &c.code).expect("event to have content")
    }
}
