use itertools::Itertools;
use log::info;
use std::{fmt::Display, sync::{Arc, RwLock}};

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
            queue.retain(|r| !r.is_similar(order));
            info!("order removed from queue: {}.", order)
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
}

#[derive(Debug)]
pub struct Order {
    pub author: String,
    pub item: String,
    pub quantity: i32,
    pub worked: RwLock<bool>,
    pub progress: RwLock<i32>,
}

impl Order {
    pub fn new(author: &str, item: &str, quantity: i32) -> Self {
        Order {
            author: author.to_owned(),
            item: item.to_owned(),
            quantity,
            worked: RwLock::new(false),
            progress: RwLock::new(0),
        }
    }

    fn is_similar(&self, other: &Order) -> bool {
        self.item == other.item
    }

    pub fn worked(&self) -> bool {
        self.worked.read().is_ok_and(|w| *w)
    }

    pub fn complete(&self) -> bool {
        self.progress() >= self.quantity
    }

    pub fn progress(&self) -> i32 {
        if let Ok(progress) = self.progress.read() {
            return *progress;
        }
        0
    }

    pub fn add_to_progress(&self, n: i32) {
        if let Ok(mut progress) = self.progress.write() {
            *progress += n;
        }
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}/{})", self.item, self.progress(), self.quantity)
    }
}
