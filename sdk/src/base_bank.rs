use crate::API;
use artifactsmmo_openapi::models::{BankSchema, SimpleItemSchema};
use std::sync::{Arc, LazyLock, RwLock};

#[derive(Default)]
pub struct BaseBank {
    pub details: RwLock<Arc<BankSchema>>,
    pub content: RwLock<Arc<Vec<SimpleItemSchema>>>,
}

pub static BASE_BANK: LazyLock<Arc<BaseBank>> = LazyLock::new(|| Arc::new(BaseBank::new()));

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

    pub fn gold(&self) -> i32 {
        self.details().gold
    }

    pub fn free_slots(&self) -> i32 {
        self.details().slots - self.content().len() as i32
    }

    pub fn total_of(&self, item: &str) -> i32 {
        self.content
            .read()
            .unwrap()
            .iter()
            .find_map(|i| {
                if i.code == item {
                    Some(i.quantity)
                } else {
                    None
                }
            })
            .unwrap_or(0)
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
