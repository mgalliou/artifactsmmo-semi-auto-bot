use itertools::Itertools;
use log::{debug, info};
use std::{
    fmt::Display,
    sync::{Arc, RwLock},
};
use strum::IntoEnumIterator;
use strum_macros::{EnumIs, EnumIter};

use super::{gear::Slot, items::Items, skill::Skill};

#[derive(Default)]
pub struct OrderBoard {
    pub orders: RwLock<Vec<Arc<Order>>>,
    items: Arc<Items>,
}

impl OrderBoard {
    pub fn new(items: &Arc<Items>) -> Self {
        OrderBoard {
            orders: RwLock::new(vec![]),
            items: items.clone(),
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

    pub fn is_ordered(&self, item: &str) -> bool {
        self.orders().iter().any(|o| o.item == item)
    }

    pub fn orders_by_priority(&self) -> Vec<Arc<Order>> {
        let mut orders: Vec<Arc<Order>> = vec![];
        Purpose::iter().for_each(|p| match p {
            Purpose::Cli => {
                orders.extend(self.orders_filtered(|o| o.purpose.is_cli()));
            }
            Purpose::Task { char: _ } => {
                orders.extend(self.orders_filtered(|o| o.purpose.is_task()));
            }
            Purpose::Gather {
                char: _,
                skill: _,
                item_code: _,
            } => {
                orders.extend(self.orders_filtered(|o| o.purpose.is_gather()));
            }
            Purpose::Food { char: _ } => {
                orders.extend(self.orders_filtered(|o| o.purpose.is_food()));
            }
            Purpose::Gear {
                char: _,
                slot: _,
                item_code: _,
            } => {
                orders.extend(self.orders_filtered(|o| o.purpose.is_gear()));
            }
            Purpose::Leveling { char: _, skill: _ } => {
                orders.extend(self.orders_filtered(|o| o.purpose.is_leveling()));
            }
        });
        orders.sort_by_key(|o| self.items.is_from_event(&o.item));
        orders
    }

    pub fn add(&self, order: Order) -> bool {
        if !self.has_similar(&order) {
            let arc = Arc::new(order);
            self.orders.write().unwrap().push(arc.clone());
            info!("orderboard: added: {}.", arc);
            return true;
        }
        false
    }

    pub fn add_or_reset(&self, order: Order) -> bool {
        if let Some(o) = self.orders().iter().find(|o| o.is_similar(&order)) {
            *o.deposited.write().unwrap() = 0;
            debug!("orderboard: reset: {}.", order);
            true
        } else {
            self.add(order)
        }
    }

    pub fn update(&self, order: Order) {
        if let Some(o) = self.orders().iter().find(|o| o.is_similar(&order)) {
            *o.quantity.write().unwrap() = order.quantity();
            debug!("orderboard: updated: {}.", order)
        } else {
            self.add(order);
        }
    }

    pub fn remove(&self, order: &Order) {
        let mut queue = self.orders.write().unwrap();
        if queue.iter().any(|r| r.is_similar(order)) {
            queue.retain(|r| !r.is_similar(order));
            info!("orderboard: removed: {}.", order)
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
    pub purpose: Purpose,
    pub worked_by: RwLock<i32>,
    pub being_crafted: RwLock<i32>,
    // Number of item deposited into the bank
    pub deposited: RwLock<i32>,
}

impl Order {
    pub fn new(owner: Option<&str>, item: &str, quantity: i32, purpose: Purpose) -> Self {
        Order {
            owner: owner.map(|o| o.to_owned()),
            item: item.to_owned(),
            quantity: RwLock::new(quantity),
            purpose,
            worked_by: RwLock::new(0),
            being_crafted: RwLock::new(0),
            deposited: RwLock::new(0),
        }
    }

    fn is_similar(&self, other: &Order) -> bool {
        self.item == other.item && self.owner == other.owner && self.purpose == other.purpose
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
        *self.deposited.read().unwrap()
    }

    pub fn quantity(&self) -> i32 {
        *self.quantity.read().unwrap()
    }

    pub fn missing(&self) -> i32 {
        self.quantity() - self.deposited()
    }

    pub fn inc_deposited(&self, n: i32) {
        *self.deposited.write().unwrap() += n;
    }

    pub fn inc_being_crafted(&self, n: i32) {
        *self.being_crafted.write().unwrap() += n;
    }

    pub fn dec_being_crafted(&self, n: i32) {
        *self.being_crafted.write().unwrap() -= n;
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

#[derive(Debug, PartialEq, Clone, EnumIs, EnumIter)]
pub enum Purpose {
    Cli,
    Gather {
        char: String,
        skill: Skill,
        item_code: String,
    },
    Food {
        char: String,
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
