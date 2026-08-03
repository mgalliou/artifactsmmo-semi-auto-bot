use crate::{
    Cached, CollectionClient, Data,
    client::events::EventsClient,
    entities::{Map, MapHandle, RawMap},
    skill::Skill,
};
use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use openapi::models::{MapContentSchema, MapContentType, MapLayer, TaskType};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

type MapsSource = Box<dyn Fn() -> HashMap<(MapLayer, i32, i32), MapHandle> + Send + Sync + 'static>;

#[derive(Clone, Deref, CollectionClient)]
#[deref(forward)]
#[key((MapLayer, i32, i32))]
#[element(MapHandle)]
pub struct MapsClient(Arc<MapsClientInner>);

pub struct MapsClientInner {
    cache_dir: Box<str>,
    data: ArcSwap<HashMap<(MapLayer, i32, i32), MapHandle>>,
    height: AtomicU32,
    width: AtomicU32,
    fetch: MapsSource,
    events: EventsClient,
}

impl MapsClient {
    #[must_use]
    pub(crate) fn new(cache_dir: &str, fetch: MapsSource, events: EventsClient) -> Self {
        Self(Arc::new(MapsClientInner {
            cache_dir: cache_dir.into(),
            data: ArcSwap::default(),
            height: AtomicU32::new(0),
            width: AtomicU32::new(0),
            fetch,
            events,
        }))
    }

    pub fn init(&self) {
        self.data.store(Arc::new(self.fetch()));
        self.init_sizes();
        info!("Maps client initialized");
    }

    fn init_sizes(&self) {
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        let data = self.data();

        for &(_, x, y) in data.keys() {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        self.width.store(span(min_x, max_x), Ordering::SeqCst);
        self.height.store(span(min_y, max_y), Ordering::SeqCst);
    }

    fn events(&self) -> EventsClient {
        self.events.clone()
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn get_raw(&self, position: &(MapLayer, i32, i32)) -> Option<RawMap> {
        Some(self.get(position)?.load())
    }

    #[must_use]
    pub fn get_by_id(&self, id: i32) -> Option<RawMap> {
        self.all_raw().into_iter().find(|m| m.id() == id)
    }

    #[must_use]
    pub fn all_raw(&self) -> Vec<RawMap> {
        self.iter().map(|map| map.load()).collect_vec()
    }

    pub fn refresh_from_events(&self) {
        for e in self.events().active() {
            if e.is_expired()
                && let Some(map) = self.get(&e.map().position())
            {
                map.store(e.previous_map());
            }
        }
        self.events().refresh_active();
        for e in self.events().active() {
            if !e.is_expired()
                && let Some(map) = self.get(&e.map().position())
            {
                map.store(e.map());
            }
        }
    }

    //TODO: handle layer
    #[must_use]
    pub fn closest_from_among(x: i32, y: i32, maps: &[RawMap]) -> Option<RawMap> {
        maps.iter()
            .min_by_key(|m| i32::abs(x - m.x()) + i32::abs(y - m.y()))
            .cloned()
    }

    #[must_use]
    pub fn of_type(&self, r#type: MapContentType) -> Vec<RawMap> {
        self.all_raw()
            .into_iter()
            .filter_map(|m| m.content_type_is(r#type).then_some(m))
            .collect_vec()
    }

    #[must_use]
    pub fn with_content_code(&self, code: &str) -> Vec<RawMap> {
        self.all_raw()
            .into_iter()
            .filter_map(|m| m.content_code_is(code).then_some(m))
            .collect()
    }

    #[must_use]
    pub fn with_content(&self, content: &MapContentSchema) -> Vec<RawMap> {
        self.all_raw()
            .into_iter()
            .filter_map(|m| m.content_is(content).then_some(m))
            .collect()
    }

    #[must_use]
    pub fn with_workshop_for(&self, skill: Skill) -> Option<RawMap> {
        match skill {
            Skill::Weaponcrafting
            | Skill::Gearcrafting
            | Skill::Jewelrycrafting
            | Skill::Cooking
            | Skill::Woodcutting
            | Skill::Mining
            | Skill::Alchemy => self.with_content_code(skill.as_ref()).first().cloned(),
            Skill::Combat | Skill::Fishing => None,
        }
    }

    #[must_use]
    pub fn closest_with_content_code_from(&self, map: &RawMap, code: &str) -> Option<RawMap> {
        let maps = self.with_content_code(code);
        map.closest_among(&maps)
    }

    fn closest_with_content_from(
        &self,
        map: &RawMap,
        content: &MapContentSchema,
    ) -> Option<RawMap> {
        let maps = self.with_content(content);
        map.closest_among(&maps)
    }

    #[must_use]
    pub fn closest_of_type_from(&self, map: &RawMap, r#type: MapContentType) -> Option<RawMap> {
        let maps = self.of_type(r#type);
        map.closest_among(&maps)
    }

    #[must_use]
    pub fn closest_tasksmaster_from(
        &self,
        map: &RawMap,
        r#type: Option<TaskType>,
    ) -> Option<RawMap> {
        r#type.map_or_else(
            || self.closest_of_type_from(map, MapContentType::TasksMaster),
            |r#type| {
                self.closest_with_content_from(
                    map,
                    &MapContentSchema {
                        r#type: MapContentType::TasksMaster,
                        code: r#type.to_string(),
                    },
                )
            },
        )
    }
}

impl Cached<HashMap<(MapLayer, i32, i32), MapHandle>> for MapsClient {
    const FILE: &'static str = "maps";

    fn cache_dir(&self) -> &str {
        &self.cache_dir
    }

    fn fetch_from_source(&self) -> HashMap<(MapLayer, i32, i32), MapHandle> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
        self.init_sizes();
    }
}

const fn span(min: i32, max: i32) -> u32 {
    if max < min {
        0
    } else {
        (max - min + 1).unsigned_abs()
    }
}

#[cfg(test)]
mod tests {
    //use super::*;

    // #[test]
    // fn check_content_type_as_string() {
    //     assert_eq!(ContentType::Monster.to_string(), "monster");
    //     assert_eq!(ContentType::Monster.as_ref(), "monster");
    // }
}
