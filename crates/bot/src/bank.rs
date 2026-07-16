use crate::{
    FOOD_CONSUMPTION_BLACKLIST,
    reservable::{Key, Reservable, ReservationError},
};
use derive_more::Deref;
use itertools::Itertools;
use sdk::{
    BankClient, Code, CollectionClient, HasDropTable, ItemContainer, ItemsClient, Level,
    LimitedContainer, Quantity, SlotLimited,
    bank::Bank,
    entities::{CharacterName, Item},
    models::{BankSchema, SimpleItemSchema},
};
use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError},
};

pub struct BankControllerInner {
    client: BankClient,
    items: ItemsClient,
    reservations: RwLock<HashMap<BankKey<String>, u32>>,
    pub browsed: RwLock<()>,
    pub being_expanded: RwLock<()>,
}

#[derive(Clone, Deref)]
#[deref(forward)]
pub struct BankController(Arc<BankControllerInner>);

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct BankKey<T = String> {
    item: T,
    owner: CharacterName,
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

    pub fn expansion_lock(
        &self,
    ) -> Result<RwLockWriteGuard<'_, ()>, TryLockError<RwLockWriteGuard<'_, ()>>> {
        self.being_expanded.try_write()
    }

    pub fn browse_lock(&self) -> RwLockWriteGuard<'_, ()> {
        self.browsed.write().unwrap()
    }

    /// Returns the quantity of each of the missing materials required to craft the `quantity` of the  item `code`
    /// for the given `owner`.
    pub fn missing_among(
        &self,
        items: &[SimpleItemSchema],
        owner: &CharacterName,
    ) -> Vec<SimpleItemSchema> {
        items
            .iter()
            .filter_map(|item| {
                let missing = item
                    .quantity
                    .saturating_sub(self.has_available((&item.code, owner)));
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

    pub fn has_all_available(&self, items: &[SimpleItemSchema], owner: &CharacterName) -> bool {
        items
            .iter()
            .all(|i| self.has_available((i.code(), owner)) >= i.quantity)
    }

    /// Returns the `quantity` of the given item `code` available to the given `owner`.
    /// If no owner is given returns the quantity not reserved.
    pub fn has_available<'a>(&self, key: impl Into<BankKey<&'a str>>) -> u32 {
        self.quantity_allowed(key)
    }

    pub fn reserve_all(
        &self,
        items: &[SimpleItemSchema],
        owner: &CharacterName,
    ) -> Result<(), ReservationError> {
        for item in items {
            self.reserve((&item.code, owner), item.quantity)?;
        }
        Ok(())
    }

    pub fn release_all(&self, items: &[SimpleItemSchema], owner: &CharacterName) {
        for item in items {
            self.release((&item.code, owner), item.quantity);
        }
    }

    /// Returns the quantity the given `owner` can withdraw from the bank.
    fn quantity_allowed<'a>(&self, key: impl Into<BankKey<&'a str>>) -> u32 {
        let key = key.into();
        self.total_of(key.item)
            .saturating_sub(self.quantity_not_allowed(&key))
    }

    /// Returns the quantity of the given item `code` that is reserved to a different character
    /// than the given `owner`.
    fn quantity_not_allowed(&self, discriminant: &BankKey<&str>) -> u32 {
        self.reservations()
            .iter()
            .filter(|(d, _)| d.code() == discriminant.item && d.owner != discriminant.owner)
            .map(|(_, q)| q)
            .sum()
    }

    pub(crate) fn available_for(&self, name: &CharacterName) -> HashMap<String, u32> {
        self.content()
            .iter()
            .map(|i| {
                (
                    i.code().to_string(),
                    self.quantity_allowed((i.code(), name.clone())),
                )
            })
            .collect()
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

    fn has_room_for_all(&self, items: &[impl Code + Quantity]) -> bool {
        self.client.has_room_for_all(items)
    }

    fn has_room_for_drops_from(&self, entity: &impl HasDropTable) -> bool {
        self.client.has_room_for_drops_from(entity)
    }
}

impl SlotLimited for BankController {
    fn free_slots(&self) -> u32 {
        self.client.free_slots()
    }
}

impl Reservable for BankController {
    type Key = BankKey<String>;

    fn reservations(&self) -> RwLockReadGuard<'_, HashMap<Self::Key, u32>> {
        self.reservations.read().unwrap()
    }

    fn reservations_mut(&self) -> RwLockWriteGuard<'_, HashMap<Self::Key, u32>> {
        self.reservations.write().unwrap()
    }
}

impl<T: AsRef<str> + Hash + Eq + Debug> Key for BankKey<T> {}

impl<T: ToString, U: Into<CharacterName>> From<(T, U)> for BankKey<String> {
    fn from(value: (T, U)) -> Self {
        Self {
            item: value.0.to_string(),
            owner: value.1.into(),
        }
    }
}

impl<'a, T: AsRef<str> + ?Sized, U: Into<CharacterName>> From<(&'a T, U)> for BankKey<&'a str> {
    fn from(value: (&'a T, U)) -> Self {
        Self {
            item: value.0.as_ref(),
            owner: value.1.into(),
        }
    }
}

impl<T: AsRef<str>> Code for BankKey<T> {
    fn code(&self) -> &str {
        self.item.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use sdk::test_utils::ITEMS;

    use super::*;

    fn bank_controller() -> BankController {
        BankController::new(BankClient::default(), ITEMS.clone())
    }

    #[test]
    fn reserv_with_not_item() {
        let bank = bank_controller();
        let result = bank.inc_reservation(("iron_ore", "char1"), 50);
        assert_eq!(Err(ReservationError::QuantityUnavailable(50)), result);
    }

    #[test]
    fn reserv_with_item_available() {
        let bank = bank_controller();
        bank.client.set_content(vec![SimpleItemSchema {
            code: "copper_ore".to_owned(),
            quantity: 100,
        }]);
        let _ = bank.inc_reservation(("copper_ore", "char1"), 50);
        let _ = bank.inc_reservation(("copper_ore", "char1"), 50);
        assert_eq!(100, bank.has_available(("copper_ore", "char1")));
    }

    #[test]
    fn reserv_if_not_with_item_available() {
        let bank = bank_controller();
        bank.client.set_content(vec![SimpleItemSchema {
            code: "gold_ore".into(),
            quantity: 100,
        }]);
        let _ = bank.reserve(("gold_ore", "char1"), 50);
        let _ = bank.reserve(("gold_ore", "char1"), 50);
        assert_eq!(100, bank.has_available(("gold_ore", "char1")));
        assert_eq!(50, bank.reserved("gold_ore"));
    }
}
