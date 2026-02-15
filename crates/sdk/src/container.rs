use crate::{Code, DropsItems, Quantity};
use openapi::models::SimpleItemSchema;
use std::sync::Arc;

pub trait ItemContainer {
    type Slot: Code + Quantity;

    fn content(&self) -> Arc<Vec<Self::Slot>>;

    fn total_items(&self) -> u32 {
        self.content().iter().map(|i| i.quantity()).sum()
    }

    fn total_of(&self, item_code: &str) -> u32 {
        self.content()
            .iter()
            .find(|i| i.code() == item_code)
            .map_or(0, |i| i.quantity())
    }

    fn contains_multiple(&self, items: &[SimpleItemSchema]) -> bool {
        items.iter().all(|i| self.total_of(&i.code) >= i.quantity)
    }
}

pub trait LimitedContainer {
    fn is_full(&self) -> bool;
    fn has_room_for_multiple(&self, items: &[SimpleItemSchema]) -> bool;
    fn has_room_for_drops_from<H: DropsItems>(&self, entity: &H) -> bool;

    fn has_room_for(&self, item_code: &str, quantity: u32) -> bool {
        self.has_room_for_multiple(&[SimpleItemSchema {
            code: item_code.to_owned(),
            quantity,
        }])
    }
}

pub trait SlotLimited: ItemContainer + LimitedContainer {
    fn free_slots(&self) -> u32;
}

pub trait SpaceLimited: ItemContainer + LimitedContainer {
    fn max_items(&self) -> u32;

    fn free_space(&self) -> u32 {
        self.max_items().saturating_sub(self.total_items())
    }
}
