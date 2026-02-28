use crate::entities::RawCharacter;
use openapi::models::CharacterSchema;
use std::sync::{Arc, RwLock};

#[derive(Default, Debug)]
pub struct CharacterDataHandle(Arc<RwLock<RawCharacter>>);

impl CharacterDataHandle {
    pub(crate) fn new(schema: CharacterSchema) -> Self {
        Self(Arc::new(RwLock::new(RawCharacter::new(schema))))
    }

    pub fn read(&self) -> RawCharacter {
        self.0.read().unwrap().clone()
    }

    pub(crate) fn update(&self, data: RawCharacter) {
        *self.0.write().unwrap() = data
    }
}
