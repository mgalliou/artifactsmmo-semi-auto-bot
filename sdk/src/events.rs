use crate::{PersistedData, API};
use artifactsmmo_openapi::models::{ActiveEventSchema, EventSchema};
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use log::debug;
use std::sync::{Arc, LazyLock, RwLock};

pub static EVENTS: LazyLock<Events> = LazyLock::new(Events::new);

pub struct Events {
    data: RwLock<Vec<Arc<EventSchema>>>,
    active: RwLock<Vec<Arc<ActiveEventSchema>>>,
    last_refresh: RwLock<DateTime<Utc>>,
}

impl PersistedData<Vec<Arc<EventSchema>>> for Events {
    const PATH: &'static str = ".cache/events.json";

    fn data_from_api() -> Vec<Arc<EventSchema>> {
        API.events
            .all()
            .unwrap()
            .into_iter()
            .map(Arc::new)
            .collect()
    }

    fn refresh_data(&self) {
        *self.data.write().unwrap() = Self::data_from_api();
    }
}

impl Events {
    fn new() -> Self {
        let events = Self {
            data: RwLock::new(Self::retrieve_data()),
            active: RwLock::new(vec![]),
            last_refresh: RwLock::new(DateTime::<Utc>::MIN_UTC),
        };
        events.refresh_active();
        events
    }

    pub fn all(&self) -> Vec<Arc<EventSchema>> {
        self.data.read().unwrap().iter().cloned().collect_vec()
    }

    pub fn active(&self) -> Vec<Arc<ActiveEventSchema>> {
        self.active.read().unwrap().iter().cloned().collect_vec()
    }

    pub fn refresh_active(&self) {
        let now = Utc::now();
        if Utc::now() - self.last_refresh() <= Duration::seconds(30) {
            return;
        }
        // NOTE: keep `events` locked before updating last refresh
        let mut events = self.active.write().unwrap();
        self.update_last_refresh(now);
        if let Ok(new) = API.events.active() {
            *events = new.into_iter().map(Arc::new).collect_vec();
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
