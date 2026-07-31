use crate::{
    Code, HasDropTable, Quantity,
    container::{ItemContainer, LimitedContainer, SlotLimited, SpaceLimited},
    entities::{Character, CharacterHandle, Item},
    gear::Slot,
    simulator::HasEffects,
};
use openapi::models::{InventorySlotSchema, SimpleItemSchema};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct InventoryClient {
    data: CharacterHandle,
}

impl InventoryClient {
    #[must_use]
    pub const fn new(data: CharacterHandle) -> Self {
        Self { data }
    }
}

pub trait Inventory: SlotLimited + SpaceLimited {
    /// Checks there is enough room to craft `item`.
    /// Returns `false` if `item` is not craftable or mats required are missing.
    fn has_room_to_craft(&self, item: &Item) -> bool {
        if !item.is_craftable() || !self.contains_all(item.mats()) {
            return false;
        }
        let free_slot = self.free_slots();
        let slot_freed = item
            .mats()
            .iter()
            .filter(|i| self.total_of(i.code()) <= i.quantity())
            .count() as i32;
        let slot_taken = i32::from(self.total_of(item.code()) == 0);
        if free_slot < 1 && slot_freed < slot_taken {
            return false;
        }
        self.free_space() >= item.craft_quantity().saturating_sub(item.mats_quantity())
    }

    /// Checks if there is enough inventory space to unequip the given items,
    /// accounting for both the items going into inventory and the
    /// `inventory_space` bonus lost from the removed gear.
    fn has_space_to_unequip(&self, items: &[(Slot, Item, u32)]) -> bool {
        let simple_items: Vec<SimpleItemSchema> = items
            .iter()
            .map(|(_, item, qty)| SimpleItemSchema::new(item.code().to_owned(), *qty))
            .collect();
        let total_quantity: i32 = items.iter().map(|(_, _, qty)| *qty as i32).sum();
        let inventory_space_lost: i32 = items
            .iter()
            .map(|(_, item, _)| item.inventory_space())
            .sum();

        self.has_room_for_all(&simple_items)
            && self.free_space() as i32 - total_quantity - inventory_space_lost > 0
    }
}

impl Inventory for InventoryClient {}

impl ItemContainer for InventoryClient {
    type Slot = InventorySlotSchema;

    fn content(&self) -> Arc<Vec<Self::Slot>> {
        self.data.load().inventory_items()
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
        self.data.load().inventory_max_items()
    }
}

impl LimitedContainer for InventoryClient {
    fn is_full(&self) -> bool {
        self.total_items() >= self.max_items() && self.free_slots() == 0
    }

    fn has_room_for_all(&self, items: &[impl Code + Quantity]) -> bool {
        let mut free_slot = self.free_slots();
        let mut free_space = self.free_space();
        for item in items {
            if free_slot < 1 || free_space < item.quantity() {
                return false;
            }
            if self.total_of(item.code()) < 1 {
                free_slot -= 1;
            }
            free_space -= item.quantity();
        }
        true
    }

    fn has_room_for_drops_from(&self, entity: &impl HasDropTable) -> bool {
        self.free_slots() >= entity.expected_slots().ceil() as u32
            && self.free_space() >= entity.expected_quantity().ceil() as u32
    }
}
