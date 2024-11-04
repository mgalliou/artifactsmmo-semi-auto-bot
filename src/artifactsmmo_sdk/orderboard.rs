use itertools::Itertools;
use log::{debug, info};
use std::{
    fmt::Display,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct OrderBoard {
    pub orders: RwLock<Vec<Arc<Order>>>,
}

impl OrderBoard {
    pub fn new() -> Self {
        OrderBoard {
            orders: RwLock::new(vec![]),
        }
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

    pub fn add(&self, order: Order) {
        if !self.has_similar(&order) {
            if let Ok(mut r) = self.orders.write() {
                info!("orderboard: added: {}.", order);
                r.push(Arc::new(order))
            }
        }
    }

    pub fn update(&self, order: Order) {
        if let Some(o) = self.orders().iter().find(|o| o.is_similar(&order)) {
            *o.quantity.write().unwrap() = order.quantity();
            debug!("orderboard: updated: {}.", order)
        } else {
            self.add(order)
        }
    }

    pub fn remove(&self, order: &Order) {
        if let Ok(mut queue) = self.orders.write() {
            if queue.iter().any(|r| r.is_similar(order)) {
                queue.retain(|r| !r.is_similar(order));
                info!("orderboard: removed: {}.", order)
            }
        }
    }

    pub fn has_similar(&self, other: &Order) -> bool {
        self.orders().iter().any(|o| o.is_similar(other))
    }

    pub fn notify_deposit(&self, code: &str, quantity: i32) {
        if let Some(order) = self.orders().iter().find(|o| o.item == code) {
            order.inc_deposited(quantity);
            if order.turned_in() {
                self.remove(order);
            }
        }
    }
}

#[derive(Debug)]
pub struct Order {
    pub owner: Option<String>,
    pub item: String,
    pub quantity: RwLock<i32>,
    pub priority: i32,
    pub reason: String,
    pub worked_by: RwLock<i32>,
    pub being_crafted: RwLock<i32>,
    // Number of item deposited into the bank
    pub deposited: RwLock<i32>,
}

impl Order {
    pub fn new(
        owner: Option<&str>,
        item: &str,
        quantity: i32,
        priority: i32,
        reason: String,
    ) -> Self {
        Order {
            owner: owner.map(|o| o.to_owned()),
            item: item.to_owned(),
            quantity: RwLock::new(quantity),
            priority,
            reason: reason.to_owned(),
            worked_by: RwLock::new(0),
            being_crafted: RwLock::new(0),
            deposited: RwLock::new(0),
        }
    }

    fn is_similar(&self, other: &Order) -> bool {
        self.item == other.item && self.owner == other.owner && self.reason == other.reason
    }

    pub fn worked_by(&self) -> i32 {
        *self.worked_by.read().unwrap()
    }

    pub fn turned_in(&self) -> bool {
        self.deposited() >= self.quantity()
    }

    pub fn being_crafted(&self) -> i32 {
        *self.being_crafted.read().unwrap()
    }

    pub fn deposited(&self) -> i32 {
        if let Ok(deposited) = self.deposited.read() {
            return *deposited;
        }
        0
    }

    pub fn quantity(&self) -> i32 {
        *self.quantity.read().unwrap()
    }

    pub fn missing(&self) -> i32 {
        self.quantity() - self.deposited()
    }

    pub fn inc_deposited(&self, n: i32) {
        if let Ok(mut deposited) = self.deposited.write() {
            *deposited += n;
        }
    }

    pub fn inc_being_crafted(&self, n: i32) {
        if let Ok(mut being_crafted) = self.being_crafted.write() {
            *being_crafted += n;
        }
    }

    pub fn dec_being_crafted(&self, n: i32) {
        if let Ok(mut being_crafted) = self.being_crafted.write() {
            *being_crafted -= n;
        }
    }

    pub fn inc_worked_by(&self, n: i32) {
        *self.worked_by.write().unwrap() += n
    }

    pub fn dec_worked_by(&self, n: i32) {
        *self.worked_by.write().unwrap() -= n
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] '{}'({}/{}), owner: {:?}, reason: {}",
            self.priority,
            self.item,
            self.deposited(),
            self.quantity(),
            self.owner,
            self.reason,
        )
    }
}
