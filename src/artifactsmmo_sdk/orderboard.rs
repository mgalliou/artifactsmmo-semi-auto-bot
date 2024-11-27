use itertools::Itertools;
use log::{debug, info};
use std::{
    fmt::Display,
    mem::discriminant,
    sync::{Arc, RwLock},
};
use strum::IntoEnumIterator;
use strum_macros::{EnumIs, EnumIter};

use super::{account::Account, gear::Slot, items::Items, skill::Skill};

#[derive(Default)]
pub struct OrderBoard {
    pub orders: RwLock<Vec<Arc<Order>>>,
    items: Arc<Items>,
    account: Arc<Account>,
}

impl OrderBoard {
    pub fn new(items: &Arc<Items>, account: &Arc<Account>) -> Self {
        OrderBoard {
            orders: RwLock::new(vec![]),
            items: items.clone(),
            account: account.clone(),
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
        Purpose::iter().for_each(|p| {
            let filtered = self.orders_filtered(|o| discriminant(&o.purpose) == discriminant(&p));
            // TODO: add sorting, by date/purpose
            orders.extend(filtered);
        });
        orders.sort_by_key(|o| !self.items.is_from_event(&o.item));
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
        let mut orders = self.orders.write().unwrap();
        if orders.iter().any(|r| r.is_similar(order)) {
            orders.retain(|r| !r.is_similar(order));
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

    pub fn should_be_turned_in(&self, order: &Order) -> bool {
        !order.turned_in()
            && self.account.available_in_inventories(&order.item) + order.in_progress()
                >= order.not_deposited()
    }

    pub fn total_missing_for(&self, order: &Order) -> i32 {
        order.not_deposited()
            - self.account.available_in_inventories(&order.item)
            - order.in_progress()
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
    pub fn new(owner: Option<&str>, item: &str, quantity: i32, purpose: Purpose) -> Self {
        Order {
            owner: owner.map(|o| o.to_owned()),
            item: item.to_owned(),
            quantity: RwLock::new(quantity),
            purpose,
            in_progress: RwLock::new(0),
            deposited: RwLock::new(0),
        }
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
