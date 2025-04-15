use crate::char::CharacterData;
use artifactsmmo_api_wrapper::ArtifactApi;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct Account {
    characters: Arc<HashMap<usize, CharacterData>>,
}

impl Account {
    pub fn new(api: &Arc<ArtifactApi>, name: &str) -> Self {
        Self {
            characters: Arc::new(
                api.account
                    .characters(name)
                    .unwrap()
                    .data
                    .into_iter()
                    .enumerate()
                    .map(|(id, data)| (id, Arc::new(RwLock::new(Arc::new(data)))))
                    .collect::<_>(),
            ),
        }
    }

    pub fn characters(&self) -> Arc<HashMap<usize, CharacterData>> {
        self.characters.clone()
    }
}
