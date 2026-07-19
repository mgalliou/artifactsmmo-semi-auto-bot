use crate::{
    FOOD_CONSUMPTION_BLACKLIST,
    reservable::{Key, Reservable, ReservationError},
};
use itertools::Itertools;
use sdk::{
    CharacterClient, Code, CollectionClient, HasDropTable, ItemContainer, ItemsClient, Level,
    LimitedContainer, Quantity, SlotLimited, SpaceLimited,
    character::Inventory,
    entities::Item,
    models::{InventorySlotSchema, SimpleItemSchema},
};
use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub struct InventoryController {
    items: ItemsClient,
    client: CharacterClient,
    reservations: RwLock<HashMap<InventoryKey<String>, u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InventoryKey<T = String>(T);

impl InventoryController {
    pub fn new(client: CharacterClient, items: ItemsClient) -> Self {
        Self {
            client,
            items,
            reservations: RwLock::new(HashMap::new()),
        }
    }

    pub fn simple_content(&self) -> Vec<SimpleItemSchema> {
        self.content()
            .iter()
            .filter(|&i| !i.code.is_empty())
            .map(|i| SimpleItemSchema {
                code: i.code.clone(),
                quantity: i.quantity(),
            })
            .collect_vec()
    }

    pub fn missing_among(&self, items: &[SimpleItemSchema]) -> Vec<SimpleItemSchema> {
        items
            .iter()
            .filter_map(|m| {
                let missing = m.quantity.saturating_sub(self.has_available(&m.code));
                (missing > 0).then(|| SimpleItemSchema {
                    code: m.code.clone(),
                    quantity: missing,
                })
            })
            .collect_vec()
    }

    pub fn consumable_food(&self) -> Vec<Item> {
        self.content()
            .iter()
            .filter_map(|i| {
                self.items.get(&i.code).filter(|i| {
                    i.is_food()
                        && i.level() <= self.client.level()
                        && !FOOD_CONSUMPTION_BLACKLIST.contains(&i.code())
                })
            })
            .collect_vec()
    }

    /// Returns the amount not reserved of the given item `code` in the `Character` inventory.
    pub fn has_available(&self, item: &str) -> u32 {
        self.total_of(item).saturating_sub(self.reserved(item))
    }

    pub fn reserve_all(&self, items: &[SimpleItemSchema]) -> Result<(), ReservationError> {
        for item in items {
            self.reserve(item.code.as_str(), item.quantity)?;
        }
        Ok(())
    }

    /// Decrease the reserved quantity of `item`
    pub fn release_all(&self, items: &[impl Code + Quantity]) {
        for item in items {
            self.release(item.code(), item.quantity());
        }
    }

    pub(crate) fn available_items(&self) -> HashMap<String, u32> {
        self.content()
            .iter()
            .map(|i| (i.code().to_string(), self.has_available(i.code())))
            .collect()
    }
}

impl Inventory for InventoryController {}

impl ItemContainer for InventoryController {
    type Slot = InventorySlotSchema;

    fn content(&self) -> Arc<Vec<Self::Slot>> {
        self.client.inventory().content()
    }
}

impl LimitedContainer for InventoryController {
    fn is_full(&self) -> bool {
        self.client.inventory().is_full()
    }

    fn has_room_for_all(&self, items: &[impl Code + Quantity]) -> bool {
        self.client.inventory().has_room_for_all(items)
    }

    fn has_room_for_drops_from(&self, entity: &impl HasDropTable) -> bool {
        self.client.inventory().has_room_for_drops_from(entity)
    }
}

impl SpaceLimited for InventoryController {
    fn max_items(&self) -> u32 {
        self.client.inventory().max_items()
    }
}

impl SlotLimited for InventoryController {
    fn free_slots(&self) -> u32 {
        self.client.inventory().free_slots()
    }
}

impl Reservable for InventoryController {
    type Key = InventoryKey<String>;

    fn reservations(&self) -> RwLockReadGuard<'_, HashMap<Self::Key, u32>> {
        self.reservations.read().unwrap()
    }

    fn reservations_mut(&self) -> RwLockWriteGuard<'_, HashMap<Self::Key, u32>> {
        self.reservations.write().unwrap()
    }
}

impl<T: AsRef<str> + Hash + Eq + Debug> Key for InventoryKey<T> {}

impl<T: ToString> From<T> for InventoryKey<String> {
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}

impl<'a, T: AsRef<str> + ?Sized> From<&'a T> for InventoryKey<&'a str> {
    fn from(value: &'a T) -> Self {
        Self(value.as_ref())
    }
}

impl<T: AsRef<str>> Code for InventoryKey<T> {
    fn code(&self) -> &str {
        self.0.as_ref()
    }
}
