use crate::{
    FOOD_CONSUMPTION_BLACKLIST,
    reservable::{Key, Reservable, ReservationError},
};
use derive_more::Deref;
use itertools::Itertools;
use sdk::{
    BankClient, Code, CollectionClient, DropsItems, ItemContainer, ItemsClient, Level,
    LimitedContainer, SlotLimited,
    bank::Bank,
    entities::Item,
    models::{BankSchema, SimpleItemSchema},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError},
};

#[derive(Default, Clone, Deref)]
#[deref(forward)]
pub struct BankController(Arc<BankControllerInner>);

#[derive(Default)]
pub struct BankControllerInner {
    client: BankClient,
    items: ItemsClient,
    reservations: RwLock<HashMap<BankKey, u32>>,
    pub browsed: RwLock<()>,
    pub being_expanded: RwLock<()>,
}

impl BankController {
    pub fn new(client: BankClient, items: ItemsClient) -> Self {
        Self(
            BankControllerInner {
                client,
                items,
                reservations: RwLock::new(HashMap::new()),
                browsed: RwLock::new(()),
                being_expanded: RwLock::new(()),
            }
            .into(),
        )
    }

    // TODO: check if this can be removed
    // Returns the quantity of the given item `code` that can be crafted with the mats available in bank
    // for the given `owner`.
    //  NOTE: this should maybe return a Option to indicate that the item is not craftable and
    //  return None in this case
    // #[deprecated]
    // pub fn has_mats_for(&self, item: &str, owner: &str) -> u32 {
    //     self.items
    //         .mats_of(item)
    //         .iter()
    //         .map(|mat| self.has_available(&mat.code, owner) / mat.quantity)
    //         .min()
    //         .unwrap_or(0)
    // }

    pub fn expension_lock(
        &self,
    ) -> Result<RwLockWriteGuard<'_, ()>, TryLockError<RwLockWriteGuard<'_, ()>>> {
        self.being_expanded.try_write()
    }

    pub fn browse_lock(&self) -> RwLockWriteGuard<'_, ()> {
        self.browsed.write().unwrap()
    }

    /// Returns the quantity of each of the missing materials required to craft the `quantity` of the  item `code`
    /// for the given `owner`.
    pub fn missing_among(&self, items: &[SimpleItemSchema], owner: &str) -> Vec<SimpleItemSchema> {
        items
            .iter()
            .filter_map(|item| {
                let missing = item
                    .quantity
                    .saturating_sub(self.has_available(&(item.code.as_str(), owner).into()));
                (missing > 0).then(|| SimpleItemSchema {
                    code: item.code.clone(),
                    quantity: missing,
                })
            })
            .collect_vec()
    }

    pub fn consumable_food(&self, level: u32) -> Vec<Item> {
        self.content()
            .iter()
            .filter_map(|i| {
                self.items.get(&i.code).filter(|i| {
                    i.is_food()
                        && i.level() <= level
                        && !FOOD_CONSUMPTION_BLACKLIST.contains(&i.code())
                })
            })
            .collect_vec()
    }

    pub fn has_multiple_available(&self, items: &[SimpleItemSchema], owner: &str) -> bool {
        items
            .iter()
            .all(|i| self.has_available(&(i.code(), owner).into()) >= i.quantity)
    }

    /// Returns the `quantity` of the given item `code` available to the given `owner`.
    /// If no owner is given returns the quantity not reserved.
    pub fn has_available(&self, discriminant: &BankKey) -> u32 {
        self.quantity_allowed(discriminant)
    }

    pub fn reserve_all(
        &self,
        items: &[SimpleItemSchema],
        owner: &str,
    ) -> Result<(), ReservationError> {
        for item in items {
            self.reserve((item.code.as_str(), owner), item.quantity)?;
        }
        Ok(())
    }

    pub fn release_all(&self, items: &[SimpleItemSchema], owner: &str) {
        for item in items {
            self.release((&item.code, owner), item.quantity);
        }
    }

    /// Returns the quantity the given `owner` can withdraw from the bank.
    fn quantity_allowed(&self, discriminant: &BankKey) -> u32 {
        self.total_of(&discriminant.item)
            .saturating_sub(self.quantity_not_allowed(discriminant))
    }

    /// Returns the quantity of the given item `code` that is reserved to a different character
    /// than the given `owner`.
    fn quantity_not_allowed(&self, discriminant: &BankKey) -> u32 {
        self.reservations()
            .iter()
            .filter(|(d, _)| d.code() == discriminant.item && d.owner != discriminant.owner)
            .map(|(_, q)| q)
            .sum()
    }
}

impl Bank for BankController {
    fn details(&self) -> Arc<BankSchema> {
        self.client.details()
    }
}

impl ItemContainer for BankController {
    type Slot = SimpleItemSchema;

    fn content(&self) -> Arc<Vec<Self::Slot>> {
        self.client.content()
    }
}

impl LimitedContainer for BankController {
    fn is_full(&self) -> bool {
        self.client.is_full()
    }

    fn has_room_for_multiple(&self, items: &[SimpleItemSchema]) -> bool {
        self.client.has_room_for_multiple(items)
    }

    fn has_room_for_drops_from<H: DropsItems>(&self, entity: &H) -> bool {
        self.client.has_room_for_drops_from(entity)
    }
}

impl SlotLimited for BankController {
    fn free_slots(&self) -> u32 {
        self.client.free_slots()
    }
}

impl Reservable for BankController {
    type Key = BankKey;

    fn reservations(&self) -> RwLockReadGuard<'_, HashMap<Self::Key, u32>> {
        self.reservations.read().unwrap()
    }

    fn reservations_mut(&self) -> RwLockWriteGuard<'_, HashMap<Self::Key, u32>> {
        self.reservations.write().unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct BankKey {
    item: String,
    owner: String,
}

impl Key for BankKey {}

impl<U, V> From<(U, V)> for BankKey
where
    U: ToString,
    V: ToString,
{
    fn from(value: (U, V)) -> Self {
        Self {
            item: value.0.to_string(),
            owner: value.1.to_string(),
        }
    }
}

impl Code for BankKey {
    fn code(&self) -> &str {
        &self.item
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserv_with_not_item() {
        let bank = BankController::default();
        let result = bank.inc_reservation(("iron_ore", "char1"), 50);
        assert_eq!(Err(ReservationError::QuantityUnavailable(50)), result);
    }

    #[test]
    fn reserv_with_item_available() {
        let bank = BankController::default();
        bank.client.set_content(vec![SimpleItemSchema {
            code: "copper_ore".to_owned(),
            quantity: 100,
        }]);
        let _ = bank.inc_reservation(("copper_ore", "char1"), 50);
        let _ = bank.inc_reservation(("copper_ore", "char1"), 50);
        assert_eq!(100, bank.has_available(&("copper_ore", "char1").into()));
    }

    #[test]
    fn reserv_if_not_with_item_available() {
        let bank = BankController::default();
        bank.client.set_content(vec![SimpleItemSchema {
            code: "gold_ore".into(),
            quantity: 100,
        }]);
        let _ = bank.reserve(("gold_ore", "char1"), 50);
        let _ = bank.reserve(("gold_ore", "char1"), 50);
        assert_eq!(100, bank.has_available(&("gold_ore", "char1").into()));
        assert_eq!(50, bank.reserved("gold_ore"));
    }
}
