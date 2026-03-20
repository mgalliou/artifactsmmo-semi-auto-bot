use crate::entities::RawCharacter;
use openapi::models::CharacterSchema;
use std::sync::{Arc, RwLock};

#[derive(Default, Debug)]
pub(crate) struct CharacterDataHandle(Arc<RwLock<RawCharacter>>);

impl CharacterDataHandle {
    pub fn read(&self) -> RawCharacter {
        self.0.read().unwrap().clone()
    }

    pub(crate) fn update(&self, data: RawCharacter) {
        *self.0.write().unwrap() = data;
    }
}

impl From<CharacterSchema> for CharacterDataHandle {
    fn from(value: CharacterSchema) -> Self {
        Self(Arc::new(RwLock::new(value.into())))
    }
}

impl From<&CharacterSchema> for CharacterDataHandle {
    fn from(value: &CharacterSchema) -> Self {
        value.clone().into()
    }
}
