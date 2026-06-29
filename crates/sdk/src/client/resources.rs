use crate::{
    CollectionClient, DropsItems, Persist,
    client::events::EventsClient,
    entities::{EventSchemaExt, Resource},
};
use api::ArtifactApi;
use arc_swap::ArcSwap;
use derive_more::Deref;
use itertools::Itertools;
use log::info;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Default, Deref, CollectionClient)]
#[deref(forward)]
#[element(Resource)]
pub struct ResourcesClient(Arc<ResourcesClientInner>);

#[derive(Default)]
pub struct ResourcesClientInner {
    api: ArtifactApi,
    data: ArcSwap<HashMap<String, Resource>>,
    events: EventsClient,
}

impl ResourcesClient {
    pub(crate) fn new(api: ArtifactApi, events: EventsClient) -> Self {
        Self(
            ResourcesClientInner {
                api,
                data: ArcSwap::default(),
                events,
            }
            .into(),
        )
    }

    pub fn init(&self) {
        self.0.data.store(Arc::new(self.load()));
        info!("Resource client initilized");
    }

    #[must_use]
    pub fn dropping(&self, item_code: &str) -> Vec<Resource> {
        self.iter()
            .filter(|r| r.drops().iter().any(|d| d.code == item_code))
            .collect_vec()
    }

    #[must_use]
    pub fn is_event(&self, code: &str) -> bool {
        self.events
            .any(|e| e.content_code().is_some_and(|cc| cc == code))
    }
}

impl Persist<HashMap<String, Resource>> for ResourcesClient {
    const PATH: &'static str = ".cache/resources.json";

    fn load_from_api(&self) -> HashMap<String, Resource> {
        self.api
            .resources
            .get_all()
            .unwrap()
            .into_iter()
            .map(|r| (r.code.clone(), Resource::new(r)))
            .collect()
    }

    fn refresh(&self) {
        self.0.data.store(Arc::new(self.load_from_api()));
    }
}
