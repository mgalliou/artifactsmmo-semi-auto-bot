use crate::API;
use artifactsmmo_openapi::models::{BankSchema, SimpleItemSchema};
use std::sync::{Arc, LazyLock, RwLock};

#[derive(Default)]
pub struct BaseBank {
    pub details: RwLock<Arc<BankSchema>>,
    pub content: RwLock<Arc<Vec<SimpleItemSchema>>>,
}

pub static BASE_BANK: LazyLock<BaseBank> = LazyLock::new(BaseBank::new);

impl BaseBank {
    pub fn new() -> Self {
        Self {
            details: RwLock::new(Arc::new(*API.bank.details().unwrap().data)),
            content: RwLock::new(Arc::new(API.bank.items(None).unwrap())),
        }
    }

    pub fn details(&self) -> Arc<BankSchema> {
        return self.details.read().unwrap().clone();
    }

    pub fn content(&self) -> Arc<Vec<SimpleItemSchema>> {
        return self.content.read().unwrap().clone();
    }

    pub fn update_details(&self, details: BankSchema) {
        *self.details.write().unwrap() = Arc::new(details)
    }

    pub fn update_content(&self, content: Vec<SimpleItemSchema>) {
        *self.content.write().unwrap() = Arc::new(content)
    }
}
