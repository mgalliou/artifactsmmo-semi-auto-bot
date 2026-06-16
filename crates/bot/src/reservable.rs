use ::std::hash::Hash;
use sdk::{Code, ItemContainer};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};
use thiserror::Error;

pub trait Reservable: ItemContainer {
    type Key: Key;

    fn reservations(&self) -> RwLockReadGuard<'_, HashMap<Self::Key, u32>>;
    fn reservations_mut(&self) -> RwLockWriteGuard<'_, HashMap<Self::Key, u32>>;

    /// Ensure at least `quantity` of this item is reserved. Idempotent.
    fn reserve(&self, key: impl Into<Self::Key>, quantity: u32) -> Result<(), ReservationError> {
        let key = key.into();
        let item_code = key.code();
        let mut reservations = self.reservations_mut();
        let quantity_reserved = *reservations.get(&key).unwrap_or(&0_u32);

        if quantity_reserved >= quantity {
            return Ok(());
        }
        let additional_needed = quantity.saturating_sub(quantity_reserved);
        let total = self.total_of(item_code);
        if quantity_reserved == 0 {
            if reservations.contains_key(&key) {
                return Err(ReservationError::ReservationAlreadyExists);
            }
            check_available(&*reservations, item_code, additional_needed, total)?;
            reservations.insert(key, additional_needed);
        } else {
            check_available(&*reservations, item_code, additional_needed, total)?;
            if let Some(r) = reservations.get_mut(&key) {
                *r += additional_needed;
            }
        }
        Ok(())
    }

    /// Increase the quantity reserved of the discriminant, or insert it if not present
    fn inc_reservation(
        &self,
        key: impl Into<Self::Key>,
        quantity: u32,
    ) -> Result<(), ReservationError> {
        let key = key.into();
        let item_code = key.code();
        let total = self.total_of(item_code);
        {
            let mut reservations = self.reservations_mut();
            check_available(&*reservations, item_code, quantity, total)?;
            if let Some(r) = reservations.get_mut(&key) {
                *r += quantity;
            } else {
                reservations.insert(key, quantity);
            }
        }
        Ok(())
    }

    fn release(&self, key: impl Into<Self::Key>, quantity: u32) {
        let discriminant = key.into();
        let mut reservations = self.reservations_mut();
        let Some(quantity_reserved) = reservations.get_mut(&discriminant) else {
            return;
        };
        *quantity_reserved = quantity_reserved.saturating_sub(quantity);
        if *quantity_reserved == 0 {
            reservations.remove(&discriminant);
        }
    }

    fn reserved(&self, item: &str) -> u32 {
        sum_reservations_for(&*self.reservations(), item)
    }

    fn is_reserved(&self, item: &str) -> bool {
        self.reserved(item) > 0
    }
}

pub trait Key: Code + Clone + Hash + Eq + Debug {}

fn sum_reservations_for(reservations: &HashMap<impl Key, u32>, item: &str) -> u32 {
    reservations
        .iter()
        .filter(|(d, _)| d.code() == item)
        .map(|(_, q)| q)
        .sum()
}

fn check_available<D: Key>(
    reservations: &HashMap<D, u32>,
    item_code: &str,
    quantity: u32,
    total: u32,
) -> Result<(), ReservationError> {
    let reserved = sum_reservations_for(reservations, item_code);
    if quantity > total.saturating_sub(reserved) {
        return Err(ReservationError::QuantityUnavailable(quantity));
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReservationError {
    #[error("Quantity unavailable: {0}")]
    QuantityUnavailable(u32),
    #[error("Reservation already exists")]
    ReservationAlreadyExists,
}
