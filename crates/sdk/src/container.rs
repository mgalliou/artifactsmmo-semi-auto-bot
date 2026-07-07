use crate::{Code, DropsItems, Quantity};
use std::sync::Arc;

pub trait ItemContainer {
    type Slot: Code + Quantity;

    fn content(&self) -> Arc<Vec<Self::Slot>>;

    fn total_items(&self) -> u32 {
        self.content().iter().map(Quantity::quantity).sum()
    }

    fn total_of(&self, item_code: &str) -> u32 {
        self.content()
            .iter()
            .find(|i| i.code() == item_code)
            .map_or(0, Quantity::quantity)
    }

    fn contains_all(&self, items: &[impl Code + Quantity]) -> bool {
        items
            .iter()
            .all(|i| self.total_of(i.code()) >= i.quantity())
    }
}

pub trait LimitedContainer: ItemContainer {
    fn is_full(&self) -> bool;
    fn has_room_for_all(&self, items: &[impl Code + Quantity]) -> bool;
    fn has_room_for_drops_from<H: DropsItems>(&self, entity: &H) -> bool;

    fn has_room_for(&self, item: impl Code + Quantity) -> bool {
        self.has_room_for_all(&[item])
    }
}

pub trait SlotLimited: LimitedContainer {
    fn free_slots(&self) -> u32;
}

pub trait SpaceLimited: LimitedContainer {
    fn max_items(&self) -> u32;

    fn free_space(&self) -> u32 {
        self.max_items().saturating_sub(self.total_items())
    }
}
