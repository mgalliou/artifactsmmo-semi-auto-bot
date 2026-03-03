use crate::{
    Code, DropsItems, container::{ItemContainer, LimitedContainer, SlotLimited, SpaceLimited},
    entities::{CharacterTrait, Item, RawCharacter},
};
use itertools::Itertools;
use openapi::models::{InventorySlot, SimpleItemSchema};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct InventoryClient {
    data: RawCharacter,
}

impl InventoryClient {
    pub fn new(data: RawCharacter) -> Self {
        Self { data }
    }
}

pub trait Inventory: SlotLimited + SpaceLimited {
    /// Checks their is enough room to craft `item`, considering the materials
    /// required are present.
    /// Returns `true` if `item` is not craftable
    fn has_room_to_craft(&self, item: &Item) -> bool {
        if !item.is_craftable()
            || self.free_slots() < 1
                && item
                    .mats()
                    .iter()
                    .all(|i| self.total_of(&i.code) > i.quantity)
                && self.total_of(item.code()) > 0
        {
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
