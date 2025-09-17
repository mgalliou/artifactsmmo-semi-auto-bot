use artifactsmmo_sdk::{CollectionClient, ItemsClient, models::SimpleItemSchema, skill::Skill};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{debug, error, info};
use std::{
    cmp::min,
    fmt::{self, Display, Formatter},
    mem::discriminant,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU32, Ordering::SeqCst},
    },
};
use strum::IntoEnumIterator;
use strum_macros::{EnumIs, EnumIter};
use thiserror::Error;

use crate::account::AccountController;

#[derive(Default)]
pub struct OrderBoard {
    orders: RwLock<Vec<Arc<Order>>>,
    items: Arc<ItemsClient>,
    account: Arc<AccountController>,
}

impl OrderBoard {
    pub fn new(items: Arc<ItemsClient>, account: Arc<AccountController>) -> Self {
        OrderBoard {
            orders: RwLock::new(vec![]),
            items,
            account,
        }
    }

    pub fn get(&self, item: &str, owner: Option<&str>, purpose: &Purpose) -> Option<Arc<Order>> {
        self.orders
            .read()
            .unwrap()
            .iter()
            .find(|o| o.owner.as_deref() == owner && o.item == item && o.purpose == *purpose)
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
                .into_iter()
                .sorted_by_key(|o| o.creation)
                .rev();
            orders.extend(filtered);
        });
        orders
    }

    pub fn add_multiple(
        &self,
        items: &[SimpleItemSchema],
        owner: Option<&str>,
        purpose: &Purpose,
    ) -> Result<(), OrderError> {
        let mut ordered: bool = false;
        for m in items.iter() {
            if self
                .add(&m.code, m.quantity, owner, purpose.clone())
                .is_ok()
            {
                ordered = true
            }
        }
        match ordered {
            true => Ok(()),
            false => Err(OrderError::AlreadyExists),
        }
    }

    pub fn add(
        &self,
        item: &str,
        quantity: u32,
        owner: Option<&str>,
        purpose: Purpose,
    ) -> Result<(), OrderError> {
        if self.items.get(item).is_none() {
            return Err(OrderError::UnknownItem);
        }
        if quantity < 1 {
            return Err(OrderError::InvalidQuantity);
        }
        if self.get(item, owner, &purpose).is_some() {
            return Err(OrderError::AlreadyExists);
        }
        let order = Order::new(owner, item, quantity, purpose)?;
        let arc = Arc::new(order);
        self.orders.write().unwrap().push(arc.clone());
        info!("orderboard: added: {arc}");
        Ok(())
    }

    pub fn add_or_reset(
        &self,
        item: &str,
        quantity: u32,
        owner: Option<&str>,
        purpose: Purpose,
    ) -> Result<(), OrderError> {
        if let Some(order) = self.get(item, owner, &purpose) {
            order.reset();
            debug!("orderboard: order reseted: {order}");
            Ok(())
        } else {
            self.add(item, quantity, owner, purpose)
        }
    }

    pub fn register_deposited_items(&self, items: &[SimpleItemSchema]) {
        items.iter().for_each(|i| {
            let mut remaining = i.quantity;
            for o in self.orders_by_priority().iter() {
                if remaining == 0 {
                    break;
                }
                if i.code == o.item {
                    let quantity = min(o.quantity(), remaining);
                    o.inc_deposited(quantity);
                    if let Some(ref owner) = o.owner
                        && let Err(e) = self.account.bank.inc_reservation(&o.item, quantity, owner)
                    {
                        error!("orderboard: failed reserving deposited item: {e}")
                    }
                    if o.turned_in() {
                        self.remove(o);
                    }
                    remaining = remaining.saturating_sub(quantity);
                }
            }
        });
    }

    pub fn remove(&self, order: &Order) {
        let mut orders = self.orders.write().unwrap();
        orders.retain(|r| !r.is_similar(order));
        info!("orderboard: order removed: {}", order);
    }

    pub fn should_be_turned_in(&self, order: &Order) -> bool {
        !order.turned_in()
            && order.missing()
                <= self.account.available_in_inventories(&order.item) + order.in_progress()
    }

    pub fn total_missing_for(&self, order: &Order) -> u32 {
        order
            .missing()
            .saturating_sub(self.account.available_in_inventories(&order.item))
            .saturating_sub(order.in_progress())
    }
}

#[derive(Debug)]
pub struct Order {
    pub item: String,
    quantity: AtomicU32,
    pub owner: Option<String>,
    pub purpose: Purpose,
    in_progress: AtomicU32,
    // Number of item deposited into the bank
    deposited: AtomicU32,
    pub creation: DateTime<Utc>,
}

impl Order {
    pub fn new(
        owner: Option<&str>,
        item: &str,
        quantity: u32,
        purpose: Purpose,
    ) -> Result<Order, OrderError> {
        if quantity == 0 {
            return Err(OrderError::InvalidQuantity);
        }
        Ok(Order {
            owner: owner.map(|o| o.to_owned()),
            item: item.to_owned(),
            quantity: AtomicU32::new(quantity),
            purpose,
            in_progress: AtomicU32::new(0),
            deposited: AtomicU32::new(0),
            creation: Utc::now(),
        })
    }

    fn is_similar(&self, other: &Order) -> bool {
        self.item == other.item && self.owner == other.owner && self.purpose == other.purpose
    }

    pub fn in_progress(&self) -> u32 {
        self.in_progress.load(SeqCst)
    }

    pub fn turned_in(&self) -> bool {
        self.deposited() >= self.quantity()
    }

    pub fn deposited(&self) -> u32 {
        self.deposited.load(SeqCst)
    }

    pub fn quantity(&self) -> u32 {
        self.quantity.load(SeqCst)
    }

    pub fn missing(&self) -> u32 {
        self.quantity().saturating_sub(self.deposited())
    }

    pub fn inc_deposited(&self, n: u32) {
        self.deposited.fetch_add(n, SeqCst);
    }

    pub fn inc_in_progress(&self, n: u32) {
        self.in_progress.fetch_add(n, SeqCst);
    }

    pub fn dec_in_progress(&self, n: u32) {
        let result = self.in_progress().saturating_sub(n);
        self.in_progress.store(result, SeqCst);
    }

    pub fn reset(&self) {
        self.deposited.store(0, SeqCst);
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: '{}'({}/{}) [{}] ({})",
            self.owner.as_ref().map_or("all", |v| v),
            self.item,
            self.deposited(),
            self.quantity(),
            self.purpose,
            self.creation.format("%H:%M:%S %d/%m/%y")
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

#[derive(Debug, PartialEq, Eq, Clone, EnumIs, EnumIter)]
pub enum Purpose {
    Food { char: String },
    Cli,
    Gear { char: String, item_code: String },
    Task { char: String },
    Leveling { char: String, skill: Skill },
}

impl Display for Purpose {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Purpose::Cli => "CLI".to_owned(),
                Purpose::Leveling { char, skill } => format!("{skill} ({char})"),
                Purpose::Food { char } => format!("food ({char})"),
                Purpose::Gear { char, item_code } => format!("'{item_code}': ({char})"),
                Purpose::Task { char } => format!("task ({char})"),
            }
        )
    }
}
