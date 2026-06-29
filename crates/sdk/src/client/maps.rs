use crate::{
    CollectionClient,
    client::events::EventsClient,
    entities::Map,
    entities::{MapDataHandle, RawMap},
    skill::Skill,
};
use api::ArtifactApi;
use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use openapi::models::{MapContentSchema, MapContentType, MapLayer, TaskType};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Default, Deref, CollectionClient)]
#[deref(forward)]
#[key((MapLayer, i32, i32))]
#[element(MapDataHandle)]
pub struct MapsClient(Arc<MapsClientInner>);

#[derive(Default)]
pub struct MapsClientInner {
    data: ArcSwap<HashMap<(MapLayer, i32, i32), MapDataHandle>>,
    events: EventsClient,
    api: ArtifactApi,
}

impl MapsClient {
    pub(crate) fn new(api: ArtifactApi, events: EventsClient) -> Self {
        Self(
            MapsClientInner {
                data: ArcSwap::default(),
                events,
                api,
            }
            .into(),
        )
    }

    pub(crate) fn init(&self) {
        self.data.store(Arc::new(
            self.api
                .maps
                .get_all()
                .unwrap()
                .into_iter()
                .map(|m| ((m.layer, m.x, m.y), m.into()))
                .collect::<HashMap<(MapLayer, i32, i32), MapDataHandle>>(),
        ));
        info!("Maps client initilized");
    }

    fn events(&self) -> EventsClient {
        self.events.clone()
    }

    #[must_use]
    pub fn get_raw(&self, position: &(MapLayer, i32, i32)) -> Option<RawMap> {
        Some(self.get(position)?.read())
    }

    #[must_use]
    pub fn all_raw(&self) -> Vec<RawMap> {
        self.iter().map(|h| h.read()).collect_vec()
    }

    pub fn refresh_from_events(&self) {
        for e in self.events().active() {
            if e.is_expired()
                && let Some(map) = CollectionClient::get(self, &e.map().position())
            {
                map.update(e.previous_map());
            }
        }
        self.events().refresh_active();
        for e in self.events().active() {
            if !e.is_expired()
                && let Some(map) = CollectionClient::get(self, &e.map().position())
            {
                map.update(e.map());
            }
        }
    }

    //TODO: handle layer
    #[must_use]
    pub fn closest_from_amoung(x: i32, y: i32, maps: &[RawMap]) -> Option<RawMap> {
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

#[cfg(test)]
mod tests {
    //use super::*;

    // #[test]
    // fn check_content_type_as_string() {
    //     assert_eq!(ContentType::Monster.to_string(), "monster");
    //     assert_eq!(ContentType::Monster.as_ref(), "monster");
    // }
}
