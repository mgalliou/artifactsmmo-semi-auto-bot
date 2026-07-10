use crate::{
    BANK_EXPANSION_SIZE, Code, HasDropTable, ItemContainer, LimitedContainer, Quantity, SlotLimited,
};
use arc_swap::ArcSwap;
use derive_more::Deref;
use openapi::models::{BankSchema, SimpleItemSchema};
use std::{sync::Arc, vec::Vec};

type FetchContent = Box<dyn Fn() -> Vec<SimpleItemSchema> + Send + Sync + 'static>;
type FetchDetails = Box<dyn Fn() -> BankSchema + Send + Sync + 'static>;

#[derive(Clone, Default, Deref)]
#[deref(forward)]
pub struct BankClient(Arc<BankClientInner>);

pub struct BankClientInner {
    details: ArcSwap<BankSchema>,
    content: ArcSwap<Vec<SimpleItemSchema>>,
    fetch_details: FetchDetails,
    fetch_content: FetchContent,
}

impl BankClient {
    #[must_use]
    pub(crate) fn new(fetch_details: FetchDetails, fetch_content: FetchContent) -> Self {
        Self(Arc::new(BankClientInner {
            details: ArcSwap::default(),
            content: ArcSwap::default(),
            fetch_content,
            fetch_details,
        }))
    }

    pub(crate) fn init(&self) {
        self.set_details((self.fetch_details)());
        self.set_content((self.fetch_content)());
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
        self.details.store(Arc::new(details));
    }

    pub fn set_content(&self, content: Vec<SimpleItemSchema>) {
        self.content.store(Arc::new(content));
    }
}

impl Default for BankClientInner {
    fn default() -> Self {
        Self {
            details: ArcSwap::default(),
            content: ArcSwap::default(),
            fetch_details: Box::new(BankSchema::default),
            fetch_content: Box::new(Vec::new),
        }
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
        self.details.load().clone()
    }
}

impl ItemContainer for BankClient {
    type Slot = SimpleItemSchema;

    fn content(&self) -> Arc<Vec<Self::Slot>> {
        self.content.load().clone()
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

    fn has_room_for_all(&self, items: &[impl Code + Quantity]) -> bool {
        let mut free_slot = self.free_slots();
        for item in items {
            if free_slot < 1 {
                return false;
            }
            if self.total_of(item.code()) < 1 {
                free_slot -= 1;
            }
        }
        true
    }

    fn has_room_for_drops_from(&self, entity: &impl HasDropTable) -> bool {
        self.free_slots() >= entity.average_item_slots()
    }
}
