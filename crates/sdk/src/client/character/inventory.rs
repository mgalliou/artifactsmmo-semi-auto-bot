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
    data: Character
}

impl InventoryClient {
    pub fn new(data: Character) -> Self {
        Self { data }
    }
}

pub trait Inventory: SlotLimited + SpaceLimited {
    fn has_room_to_craft(&self, item: &Item) -> bool {
        let Some(quantity) = item
            .craft_schema()
            .and_then(|s| s.quantity.map(|q| q as u32))
        else {
            return true;
        };
        let extra_quantity = quantity.saturating_sub(item.mats_quantity());
        if extra_quantity > 0 && self.free_space() < extra_quantity
            || (self.free_slots() < 1
                && item
                    .mats()
                    .iter()
                    .all(|i| self.total_of(&i.code) > i.quantity))
        {
            return false;
        }
        true
    }
}

impl Inventory for InventoryClient {}

impl ItemContainer for InventoryClient {
    type Slot = InventorySlot;

    fn content(&self) -> Arc<Vec<InventorySlot>> {
        Arc::new(self.data.inventory().iter().flatten().cloned().collect_vec())
    }
}

impl SpaceLimited for InventoryClient {
    fn max_items(&self) -> u32 {
        self.data.inventory_max_items() as u32
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

impl LimitedContainer for InventoryClient {
    fn is_full(&self) -> bool {
        self.total_items() >= self.max_items() || self.free_slots() == 0
    }

    fn has_room_for_multiple(&self, items: &[SimpleItemSchema]) -> bool {
        let mut free_slot = self.free_slots();
        let mut free_space = self.free_space();
        for item in items.iter() {
            if free_slot < 1 || free_space < item.quantity {
                return false;
            }
            if self.total_of(&item.code) < 1 {
                free_slot -= 1
            }
            free_space -= item.quantity
        }
        true
    }

    fn has_room_for_drops_from<H: DropsItems>(&self, entity: &H) -> bool {
        self.free_slots() >= entity.average_drop_slots()
            && self.free_space() >= entity.min_drop_quantity()
    }
}
