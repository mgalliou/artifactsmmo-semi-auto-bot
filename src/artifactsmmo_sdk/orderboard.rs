use itertools::Itertools;
use log::info;
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

    // TODO: when order with same item already exist, increase existing order quantity
    pub fn order_item(&self, author: &str, item: &str, quantity: i32) {
        let request = Order::new(author, item, quantity);
        if !self.has_similar_order(&request) {
            if let Ok(mut r) = self.orders.write() {
                info!("order added to queue: {}.", request);
                r.push(Arc::new(request))
            }
        }
    }

    pub fn remove_order(&self, order: &Order) {
        if let Ok(mut queue) = self.orders.write() {
            if queue.iter().any(|r| r.is_similar(order)) {
                queue.retain(|r| !r.is_similar(order));
                info!("order removed from queue: {}.", order)
            }
        }
    }

    pub fn has_similar_order(&self, other: &Order) -> bool {
        match self.orders.read() {
            Ok(queue) => {
                return queue.iter().any(|r| r.is_similar(other));
            }
            _ => false,
        }
    }

    pub fn notify_deposit(&self, code: &str, quantity: i32) {
        if let Some(order) = self.orders().iter().find(|o| o.item == code) {
            order.inc_deposited(quantity);
            if order.turned_in() {
                self.remove_order(order);
            }
        }
    }
}

#[derive(Debug)]
pub struct Order {
    pub author: String,
    pub item: String,
    pub quantity: i32,
    pub worked: RwLock<bool>,
    pub being_crafted: RwLock<i32>,
    // Number of item deposited into the bank
    pub deposited: RwLock<i32>,
}

impl Order {
    pub fn new(author: &str, item: &str, quantity: i32) -> Self {
        Order {
            author: author.to_owned(),
            item: item.to_owned(),
            quantity,
            worked: RwLock::new(false),
            being_crafted: RwLock::new(0),
            deposited: RwLock::new(0),
        }
    }

    fn is_similar(&self, other: &Order) -> bool {
        self.item == other.item
    }

    pub fn worked(&self) -> bool {
        self.worked.read().is_ok_and(|w| *w)
    }

    pub fn turned_in(&self) -> bool {
        self.deposited() >= self.quantity
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

    pub fn missing(&self) -> i32 {
        self.quantity - self.deposited()
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
}

impl Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.item, self.quantity)
    }
}
