use crate::{BANK_EXPANSION_SIZE, ItemContainer, LimitedContainer, SlotLimited};
use api::ArtifactApi;
use derive_more::Deref;
use openapi::models::{BankSchema, SimpleItemSchema};
use std::sync::{Arc, RwLock};

#[derive(Default, Debug, Clone, Deref)]
#[deref(forward)]
pub struct BankClient(Arc<BankClientInner>);

#[derive(Default, Debug)]
pub struct BankClientInner {
    details: RwLock<Arc<BankSchema>>,
    content: RwLock<Arc<Vec<SimpleItemSchema>>>,
    api: ArtifactApi,
}

impl BankClient {
    pub(crate) fn new(api: ArtifactApi) -> Self {
        Self(
            BankClientInner {
                details: RwLock::default(),
                content: RwLock::default(),
                api,
            }
            .into(),
        )
    }

    pub(crate) fn init(&self) {
        self.set_details(self.api.bank.get_details().unwrap());
        self.set_content(self.api.bank.get_items().unwrap());
    }

    pub fn set_gold(&self, gold: u32) {
        let mut new_details = (*self.details()).clone();
        new_details.gold = gold;
        self.set_details(new_details);
    }

    pub fn expand(&self) {
        let mut new_details = (*self.details()).clone();
        new_details.slots += BANK_EXPANSION_SIZE;
        new_details.expansions += 1;
        self.set_details(new_details);
    }

    pub fn set_details(&self, details: BankSchema) {
        *self.details.write().unwrap() = Arc::new(details);
    }

    pub fn set_content(&self, content: Vec<SimpleItemSchema>) {
        *self.content.write().unwrap() = Arc::new(content);
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
        self.details.read().unwrap().clone()
    }
}

impl ItemContainer for BankClient {
    type Slot = SimpleItemSchema;

    fn content(&self) -> Arc<Vec<Self::Slot>> {
        self.content.read().unwrap().clone()
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
        for item in items {
            if free_slot < 1 {
                return false;
            }
            if self.total_of(&item.code) < 1 {
                free_slot -= 1;
            }
        }
        true
    }

    fn has_room_for_drops_from<H: crate::DropsItems>(&self, entity: &H) -> bool {
        self.free_slots() >= entity.average_drop_slots()
    }
}
