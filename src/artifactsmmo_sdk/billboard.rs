use itertools::Itertools;
use log::info;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct OrderBoard {
    pub orders: RwLock<Vec<Arc<RwLock<Order>>>>,
}

impl OrderBoard {
    pub fn new() -> Self {
        OrderBoard {
            orders: RwLock::new(vec![]),
        }
    }

    pub fn orders(&self) -> Vec<Arc<RwLock<Order>>> {
        self.orders.read().unwrap().iter().cloned().collect_vec()
    }

    pub fn order_item(&self, author: &str, item: &str, quantity: i32) {
        let request = Order::new(author, item, quantity);
        if !self.has_similar_order(&request) {
            if let Ok(mut r) = self.orders.write() {
                info!("request added to queue {:?}.", request);
                r.push(Arc::new(RwLock::new(request)))
            }
        }
    }

    pub fn remove_order(&self, order: &Order) {
        if let Ok(mut queue) = self.orders.write() {
            queue.retain(|r| *r.read().unwrap() != *order);
            info!("request removed from queue {:?}", order)
        }
    }

    pub fn has_similar_order(&self, other: &Order) -> bool {
        match self.orders.read() {
            Ok(queue) => {
                return queue
                    .iter()
                    .any(|r| r.read().is_ok_and(|r| r.is_similar(other)));
            }
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Order {
    pub author: String,
    pub item: String,
    pub quantity: i32,
    pub worked: bool,
}

impl Order {
    pub fn new(author: &str, item: &str, quantity: i32) -> Self {
        Order {
            author: author.to_owned(),
            item: item.to_owned(),
            quantity,
            worked: false,
        }
    }

    fn is_similar(&self, other: &Order) -> bool {
        self.item == other.item
    }
}
