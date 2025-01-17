use crate::{
    api::EventsApi, game_config::GAME_CONFIG, maps::MapSchemaExt, persist_data, retreive_data,
};
use artifactsmmo_openapi::models::{ActiveEventSchema, EventSchema, MapSchema};
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{debug, error};
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

lazy_static! {
    pub static ref EVENTS: Arc<Events> = Arc::new(Events::new());
}

pub struct Events {
    api: EventsApi,
    pub data: Vec<EventSchema>,
    pub active: Arc<RwLock<Vec<ActiveEventSchema>>>,
    last_refresh: RwLock<DateTime<Utc>>,
}

impl Events {
    fn new() -> Self {
        let api = EventsApi::new(&GAME_CONFIG.base_url);
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
            active: Arc::new(RwLock::new(vec![])),
            last_refresh: RwLock::new(DateTime::<Utc>::MIN_UTC),
        };
        events.refresh_active();
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

    pub fn refresh_active(&self) {
        let now = Utc::now();
        if Utc::now() - self.last_refresh() <= Duration::seconds(30) {
            return;
        }
        // NOTE: keep `events` locked before updating last refresh
        let mut events = self.active.write().unwrap();
        self.update_last_refresh(now);
        if let Ok(new) = self.api.active() {
            *events = new;
            debug!("events refreshed.");
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

impl EventSchemaExt for ActiveEventSchema {
    fn content_code(&self) -> &String {
        self.map
            .content
            .as_ref()
            .map(|c| &c.code)
            .expect("event to have content")
    }

    fn to_string(&self) -> String {
        let remaining = if let Ok(expiration) = DateTime::parse_from_rfc3339(&self.expiration) {
            (expiration.to_utc() - Utc::now()).num_seconds().to_string()
        } else {
            "?".to_string()
        };
        format!(
            "{} ({},{}): '{}', duration: {}, created at {}, expires at {}, remaining: {}s",
            self.name,
            self.map.x,
            self.map.y,
            self.content_code(),
            self.duration,
            self.created_at,
            self.expiration,
            remaining
        )
    }
}

pub trait EventSchemaExt {
    fn content_code(&self) -> &String;
    fn to_string(&self) -> String;
}

impl EventSchemaExt for EventSchema {
    fn content_code(&self) -> &String {
        &self.content.code
    }

    fn to_string(&self) -> String {
        format!("{}: '{}'", self.name, self.content_code())
    }
}
