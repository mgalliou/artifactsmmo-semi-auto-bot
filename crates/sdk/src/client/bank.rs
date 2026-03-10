use crate::{BANK_EXTENSION_SIZE, ItemContainer, LimitedContainer, SlotLimited};
use api::ArtifactApi;
use openapi::models::{BankSchema, SimpleItemSchema};
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone)]
pub struct BankClient(Arc<BankClientInner>);

#[derive(Default, Debug)]
pub struct BankClientInner {
    pub details: RwLock<Arc<BankSchema>>,
    pub content: RwLock<Arc<Vec<SimpleItemSchema>>>,
}

impl BankClient {
    pub(crate) fn new(api: Arc<ArtifactApi>) -> Self {
        Self(Arc::new(BankClientInner {
            details: RwLock::new(api.bank.get_details().unwrap().into()),
            content: RwLock::new(api.bank.get_items().unwrap().into()),
        }))
    }

    pub(crate) fn update_gold(&self, gold: u32) {
        let mut new_details = self.details().deref().clone();
        new_details.gold = gold;
        self.update_details(new_details);
    }

    pub(crate) fn expand(&self) {
        let mut new_details = self.details().deref().clone();
        new_details.slots += BANK_EXTENSION_SIZE;
        self.update_details(new_details);
    }

    pub(crate) fn update_details(&self, details: BankSchema) {
        *self.0.details.write().unwrap() = Arc::new(details)
    }

    pub(crate) fn update_content(&self, content: Vec<SimpleItemSchema>) {
        *self.0.content.write().unwrap() = Arc::new(content)
    }
}

pub trait Bank: SlotLimited {
    fn details(&self) -> Arc<BankSchema>;

    fn slots(&self) -> u32 {
        self.details().slots
    }

    fn expansions(&self) -> u32 {
        self.details().expansions
    }

    fn next_expansion_cost(&self) -> u32 {
        self.details().next_expansion_cost
    }

    fn gold(&self) -> u32 {
        self.details().gold
    }
}

impl Bank for BankClient {
    fn details(&self) -> Arc<BankSchema> {
        self.0.details.read().unwrap().clone()
    }
}

impl ItemContainer for BankClient {
    type Slot = SimpleItemSchema;

    fn content(&self) -> Arc<Vec<SimpleItemSchema>> {
        self.0.content.read().unwrap().clone()
    }
}

impl SlotLimited for BankClient {
    fn free_slots(&self) -> u32 {
        self.details()
            .slots
            .saturating_sub(self.content().len() as u32)
    }
}

impl LimitedContainer for BankClient {
    fn is_full(&self) -> bool {
        self.free_slots() == 0
    }

    fn has_room_for_multiple(&self, items: &[SimpleItemSchema]) -> bool {
        let mut free_slot = self.free_slots();
        for item in items.iter() {
            if free_slot < 1 {
                return false;
            }
            if self.total_of(&item.code) < 1 {
                free_slot -= 1
            }
        }
        true
    }

    fn has_room_for_drops_from<H: crate::DropsItems>(&self, entity: &H) -> bool {
        self.free_slots() >= entity.average_drop_slots()
    }
}
