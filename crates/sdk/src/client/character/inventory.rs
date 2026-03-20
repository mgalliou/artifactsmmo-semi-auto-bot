use crate::{
    Code, DropsItems,
    container::{ItemContainer, LimitedContainer, SlotLimited, SpaceLimited},
    entities::{Character, Item, RawCharacter},
};
use itertools::Itertools;
use openapi::models::{InventorySlot, SimpleItemSchema};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct InventoryClient {
    data: RawCharacter,
}

impl InventoryClient {
    pub const fn new(data: RawCharacter) -> Self {
        Self { data }
    }
}

pub trait Inventory: SlotLimited + SpaceLimited {
    /// Checks there is enough room to craft `item`.
    /// Returns `false` if `item` is not craftable or mats required are missing.
    fn has_room_to_craft(&self, item: &Item) -> bool {
        if !item.is_craftable() || !self.contains_multiple(&item.mats()) {
            return false;
        }
        let free_slot = self.free_slots();
        let slot_freed = item
            .mats()
            .iter()
            .filter(|i| self.total_of(&i.code) <= i.quantity)
            .count() as i32;
        let slot_taken = i32::from(self.total_of(item.code()) == 0);
        if free_slot < 1 && slot_freed < slot_taken {
            return false;
        }
        self.free_space() >= item.craft_quantity().saturating_sub(item.mats_quantity())
    }
}

impl Inventory for InventoryClient {}

impl ItemContainer for InventoryClient {
    type Slot = InventorySlot;

    fn content(&self) -> Arc<Vec<Self::Slot>> {
        Arc::new(
            self.data
                .inventory_items()
                .iter()
                .flatten()
                .cloned()
                .collect_vec(),
        )
    }
}

impl SlotLimited for InventoryClient {
    fn free_slots(&self) -> u32 {
        self.content()
            .iter()
            .filter(|i| i.code().is_empty())
            .count() as u32
    }
}

impl SpaceLimited for InventoryClient {
    fn max_items(&self) -> u32 {
        self.data.inventory_max_items() as u32
    }
}

impl LimitedContainer for InventoryClient {
    fn is_full(&self) -> bool {
        self.total_items() >= self.max_items() || self.free_slots() == 0
    }

    fn has_room_for_multiple(&self, items: &[SimpleItemSchema]) -> bool {
        let mut free_slot = self.free_slots();
        let mut free_space = self.free_space();
        for item in items {
            if free_slot < 1 || free_space < item.quantity {
                return false;
            }
            if self.total_of(&item.code) < 1 {
                free_slot -= 1;
            }
            free_space -= item.quantity;
        }
        true
    }

    fn has_room_for_drops_from<H: DropsItems>(&self, entity: &H) -> bool {
        self.free_slots() >= entity.average_drop_slots()
            && self.free_space() >= entity.min_drop_quantity()
    }
}
