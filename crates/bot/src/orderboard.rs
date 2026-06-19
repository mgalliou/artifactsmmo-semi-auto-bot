use crate::{account::AccountController, reservable::Reservable};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{debug, error, info};
use sdk::{
    Code, CollectionClient, ItemsClient, Quantity, entities::CharacterName,
    models::SimpleItemSchema, skill::Skill,
};
use std::{
    borrow::ToOwned,
    cmp::min,
    convert::Into,
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

#[derive(Default, Clone)]
pub struct OrderBoard {
    orders: Arc<RwLock<Vec<Arc<Order>>>>,
    items: ItemsClient,
    account: AccountController,
}

impl OrderBoard {
    pub fn new(items: ItemsClient, account: AccountController) -> Self {
        Self {
            orders: RwLock::default().into(),
            items,
            account,
        }
    }

    pub fn get(
        &self,
        item: &str,
        owner: Option<&CharacterName>,
        purpose: &Purpose,
    ) -> Option<Arc<Order>> {
        self.orders
            .read()
            .unwrap()
            .iter()
            .find(|o| o.owner.as_ref() == owner && o.item == item && o.purpose == *purpose)
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

    pub fn quantity_ordered(&self, item: &str) -> u32 {
        self.orders()
            .iter()
            .filter(|o| o.item == item)
            .map(|o| o.quantity())
            .sum()
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
        owner: Option<&CharacterName>,
        purpose: &Purpose,
    ) -> Result<(), OrderError> {
        let mut ordered: bool = false;
        for m in items {
            if self
                .add(&m.code, m.quantity, owner, purpose.clone())
                .is_ok()
            {
                ordered = true;
            }
        }
        if ordered {
            Ok(())
        } else {
            Err(OrderError::AlreadyExists)
        }
    }

    pub fn add(
        &self,
        item: &str,
        quantity: u32,
        owner: Option<&CharacterName>,
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
        let arc = Arc::new(Order::new(owner, item, quantity, purpose)?);
        self.orders.write().unwrap().push(arc.clone());
        info!("orderboard: added: {arc}");
        Ok(())
    }

    pub fn add_or_reset(
        &self,
        item: &str,
        quantity: u32,
        owner: Option<&CharacterName>,
        purpose: Purpose,
    ) -> Result<(), OrderError> {
        if let Some(order) = self.get(item, owner, &purpose) {
            if order.quantity() < quantity {
                order.reset();
                debug!("orderboard: order reseted: {order}");
                return Ok(());
            }
            Err(OrderError::AlreadyExists)
        } else {
            self.add(item, quantity, owner, purpose)
        }
    }

    pub fn register_deposited_items(&self, items: &[SimpleItemSchema]) {
        for item in items {
            let mut remaining = item.quantity();
            for order in &self.orders_by_priority() {
                if remaining < 1 {
                    break;
                }
                if item.code() != order.item {
                    continue;
                }
                let quantity = min(order.quantity(), remaining);
                order.inc_deposited(quantity);
                if let Some(ref owner) = order.owner
                    && let Err(e) = self
                        .account
                        .bank()
                        .inc_reservation((&order.item, owner), quantity)
                {
                    error!("orderboard: failed reserving deposited item: {e}");
                }
                if order.turned_in() {
                    self.remove(order);
                }
                remaining = remaining.saturating_sub(quantity);
            }
        }
    }

    pub fn clear(&self) {
        self.orders.write().unwrap().clear();
    }

    pub fn remove(&self, order: &Order) {
        self.orders
            .write()
            .unwrap()
            .retain(|r| !r.is_similar(order));
        info!("orderboard: order removed: {order}");
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
    pub owner: Option<CharacterName>,
    pub purpose: Purpose,
    in_progress: AtomicU32,
    // Number of item deposited into the bank
    deposited: AtomicU32,
    pub creation: DateTime<Utc>,
}

impl Order {
    pub fn new(
        owner: Option<&CharacterName>,
        item: &str,
        quantity: u32,
        purpose: Purpose,
    ) -> Result<Self, OrderError> {
        if quantity == 0 {
            return Err(OrderError::InvalidQuantity);
        }
        Ok(Self {
            owner: owner.map(Into::into),
            item: item.to_owned(),
            quantity: AtomicU32::new(quantity),
            purpose,
            in_progress: AtomicU32::new(0),
            deposited: AtomicU32::new(0),
            creation: Utc::now(),
        })
    }

    fn is_similar(&self, other: &Self) -> bool {
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
    Food {
        char: CharacterName,
    },
    Cli,
    Gear {
        char: CharacterName,
        item_code: String,
    },
    Task {
        char: CharacterName,
    },
    Leveling {
        char: CharacterName,
        skill: Skill,
    },
}

impl Display for Purpose {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Cli => "CLI".to_owned(),
                Self::Leveling { char, skill } => format!("{skill} ({char})"),
                Self::Food { char } => format!("food ({char})"),
                Self::Gear { char, item_code } => format!("'{item_code}': ({char})"),
                Self::Task { char } => format!("task ({char})"),
            }
        )
    }
}
