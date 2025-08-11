use artifactsmmo_sdk::{char::Skill, gear::Slot, Items};
use itertools::Itertools;
use log::{debug, info};
use std::{
    fmt::{self, Display, Formatter},
    mem::discriminant,
    sync::{Arc, RwLock},
};
use strum::IntoEnumIterator;
use strum_macros::{EnumIs, EnumIter};
use thiserror::Error;

use crate::account::AccountController;


#[derive(Default)]
pub struct OrderBoard {
    orders: RwLock<Vec<Arc<Order>>>,
    items: Arc<Items>,
    account: Arc<AccountController>
}

impl OrderBoard {
    pub fn new(items: Arc<Items>, account: Arc<AccountController>) -> Self {
        OrderBoard {
            orders: RwLock::new(vec![]),
            items,
            account,
        }
    }

    pub fn get(&self, owner: Option<&str>, item: &str, purpose: &Purpose) -> Option<Arc<Order>> {
        self.orders
            .read()
            .unwrap()
            .iter()
            .find(|o| {
                o.owner == owner.map(str::to_string) && o.item == item && o.purpose == *purpose
            })
            .cloned()
    }

    pub fn orders(&self) -> Vec<Arc<Order>> {
        self.orders.read().unwrap().iter().cloned().collect_vec()
    }

    pub fn orders_filtered<F>(&self, f: F) -> Vec<Arc<Order>>
    where
        F: FnMut(&Arc<Order>) -> bool,
    {
        self.orders().into_iter().filter(f).collect_vec()
    }

    pub fn is_ordered(&self, item: &str) -> bool {
        self.orders().iter().any(|o| o.item == item)
    }

    pub fn orders_by_priority(&self) -> Vec<Arc<Order>> {
        let mut orders: Vec<Arc<Order>> = vec![];
        Purpose::iter().for_each(|p| {
            let filtered = self
                .orders_filtered(|o| discriminant(&o.purpose) == discriminant(&p))
                .iter()
                .cloned()
                .chunk_by(|o| o.purpose.clone())
                .into_iter()
                .flat_map(|(_, chunk)| {
                    chunk
                        .sorted_by_key(|o| self.items.get(&o.item).map(|i| i.level).unwrap_or(1))
                        .rev()
                })
                .collect_vec();
            orders.extend(filtered);
        });
        orders.sort_by_key(|o| !self.items.is_from_event(&o.item));
        orders
    }

    pub fn add(
        &self,
        owner: Option<&str>,
        item: &str,
        quantity: i32,
        purpose: Purpose,
    ) -> Result<(), OrderError> {
        if self.items.get(item).is_none() {
            return Err(OrderError::UnknownItem);
        }
        if self.get(owner, item, &purpose).is_some() {
            return Err(OrderError::AlreadyExists);
        }
        let order = Order::new(owner, item, quantity, purpose)?;
        let arc = Arc::new(order);
        self.orders.write().unwrap().push(arc.clone());
        info!("orderboard: added: {}.", arc);
        Ok(())
    }

    pub fn add_or_reset(
        &self,
        owner: Option<&str>,
        item: &str,
        quantity: i32,
        purpose: Purpose,
    ) -> Result<(), OrderError> {
        if let Some(o) = self.get(owner, item, &purpose) {
            *o.deposited.write().unwrap() = 0;
            debug!("orderboard: reset: {}.", o);
            Ok(())
        } else {
            self.add(owner, item, quantity, purpose)
        }
    }

    pub fn register_deposit(
        &self,
        owner: &Option<String>,
        item: &str,
        quantity: i32,
        purpose: &Purpose,
    ) -> Result<(), OrderError> {
        let Some(order) = self.get(owner.as_deref(), item, purpose) else {
            return Err(OrderError::NotFound);
        };
        order.inc_deposited(quantity);
        if order.turned_in() {
            self.remove(&order)?;
        }
        Ok(())
    }

    pub fn remove(&self, order: &Order) -> Result<(), OrderError> {
        let mut orders = self.orders.write().unwrap();
        if orders.iter().any(|r| r.is_similar(order)) {
            orders.retain(|r| !r.is_similar(order));
            info!("orderboard: removed: {}.", order);
            Ok(())
        } else {
            Err(OrderError::NotFound)
        }
    }

    pub fn should_be_turned_in(&self, order: &Order) -> bool {
        !order.turned_in()
            && self.account.available_in_inventories(&order.item) + order.in_progress()
                >= order.not_deposited()
    }

    pub fn total_missing_for(&self, order: &Order) -> i32 {
        order.not_deposited() - self.account.available_in_inventories(&order.item) - order.in_progress()
    }
}

#[derive(Debug)]
pub struct Order {
    pub owner: Option<String>,
    pub item: String,
    pub quantity: RwLock<i32>,
    pub purpose: Purpose,
    pub in_progress: RwLock<i32>,
    // Number of item deposited into the bank
    pub deposited: RwLock<i32>,
}

impl Order {
    pub fn new(
        owner: Option<&str>,
        item: &str,
        quantity: i32,
        purpose: Purpose,
    ) -> Result<Order, OrderError> {
        if quantity <= 0 {
            return Err(OrderError::InvalidQuantity);
        }
        Ok(Order {
            owner: owner.map(|o| o.to_owned()),
            item: item.to_owned(),
            quantity: RwLock::new(quantity),
            purpose,
            in_progress: RwLock::new(0),
            deposited: RwLock::new(0),
        })
    }

    fn is_similar(&self, other: &Order) -> bool {
        self.item == other.item && self.owner == other.owner && self.purpose == other.purpose
    }

    pub fn in_progress(&self) -> i32 {
        *self.in_progress.read().unwrap()
    }

    pub fn turned_in(&self) -> bool {
        self.deposited() >= self.quantity()
    }

    pub fn deposited(&self) -> i32 {
        *self.deposited.read().unwrap()
    }

    pub fn quantity(&self) -> i32 {
        *self.quantity.read().unwrap()
    }

    pub fn not_deposited(&self) -> i32 {
        self.quantity() - self.deposited()
    }

    pub fn inc_deposited(&self, n: i32) {
        *self.deposited.write().unwrap() += n;
    }

    pub fn inc_in_progress(&self, n: i32) {
        *self.in_progress.write().unwrap() += n;
    }

    pub fn dec_in_progress(&self, n: i32) {
        *self.in_progress.write().unwrap() -= n;
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: '{}'({}/{}), purpose: {}",
            if let Some(owner) = &self.owner {
                owner
            } else {
                "all"
            },
            self.item,
            self.deposited(),
            self.quantity(),
            self.purpose,
        )
    }
}

#[derive(Debug, Error)]
pub enum OrderError {
    #[error("invalid quantity")]
    InvalidQuantity,
    #[error("order not found")]
    NotFound,
    #[error("order already exists")]
    AlreadyExists,
    #[error("unknown item")]
    UnknownItem,
}

#[derive(Debug, PartialEq, Clone, EnumIs, EnumIter)]
pub enum Purpose {
    Food {
        char: String,
    },
    Cli,
    Gather {
        char: String,
        skill: Skill,
        item_code: String,
    },
    Gear {
        char: String,
        slot: Slot,
        item_code: String,
    },
    Task {
        char: String,
    },
    Leveling {
        char: String,
        skill: Skill,
    },
}

impl Display for Purpose {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Purpose::Cli => "command line".to_owned(),
                Purpose::Leveling { char, skill } => format!("leveling {char}'s {skill}"),
                Purpose::Gather {
                    char,
                    skill,
                    item_code,
                } => format!("{char}'s '{item_code}' ({skill})"),
                Purpose::Food { char } => format!("{char}'s food"),
                Purpose::Gear {
                    char,
                    slot,
                    item_code,
                } => format!("{char}'s '{item_code}' ({slot})"),
                Purpose::Task { char } => format!("{char}'s  task"),
            }
        )
    }
}
