use super::{
    api::events::EventsApi, game_config::GameConfig, persist_data, retreive_data,
    ActiveEventSchemaExt, MapSchemaExt,
};
use artifactsmmo_openapi::models::{ActiveEventSchema, EventSchema, MapSchema};
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use log::{debug, error};
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

pub struct Events {
    api: EventsApi,
    pub data: Vec<EventSchema>,
    pub active: RwLock<Vec<ActiveEventSchema>>,
    last_refresh: RwLock<DateTime<Utc>>,
}

impl Events {
    pub fn new(config: &GameConfig) -> Self {
        let api = EventsApi::new(&config.base_url);
        let path = Path::new(".cache/events.json");
        let data = if let Ok(data) = retreive_data::<Vec<EventSchema>>(path) {
            data
        } else {
            let data = api.all().expect("items to be retrieved from API.");
            if let Err(e) = persist_data(&data, path) {
                error!("failed to persist items data: {}", e);
            }
            data
        };
        let events = Self {
            api,
            data,
            active: RwLock::new(vec![]),
            last_refresh: RwLock::new(DateTime::<Utc>::MIN_UTC),
        };
        events.refresh();
        events
    }

    pub fn maps(&self) -> Vec<Arc<MapSchema>> {
        self.active
            .read()
            .unwrap()
            .iter()
            .map(|e| Arc::new(*e.map.clone()))
            .collect_vec()
    }

    pub fn refresh(&self) {
        let now = Utc::now();
        if Utc::now() - self.last_refresh() > Duration::seconds(30) {
            // NOTE: keep `events` locked before updating last refresh
            let mut events = self.active.write().unwrap();
            self.update_last_refresh(now);
            if let Ok(new) = self.api.active() {
                *events = new;
                debug!("events refreshed.");
            }
        }
    }

    fn update_last_refresh(&self, now: DateTime<Utc>) {
        self.last_refresh
            .write()
            .expect("`last_refresh` to be writable")
            .clone_from(&now);
    }

    pub fn last_refresh(&self) -> DateTime<Utc> {
        *self
            .last_refresh
            .read()
            .expect("`last_refresh` to be readable")
    }

    pub fn of_type(&self, r#type: &str) -> Vec<ActiveEventSchema> {
        self.active
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.map.content().is_some_and(|c| c.r#type == r#type))
            .cloned()
            .collect_vec()
    }
}

impl ActiveEventSchemaExt for ActiveEventSchema {
    fn content_code(&self) -> &String {
        self.map
            .content
            .as_ref()
            .map(|c| &c.code)
            .expect("event to have content")
    }
}
