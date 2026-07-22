use crate::{Code, HasDropTable, Quantity};
use std::sync::Arc;

pub trait ItemContainer {
    type Slot: Code + Quantity;

    fn content(&self) -> Arc<Vec<Self::Slot>>;

    fn total_items(&self) -> u32 {
        self.content().iter().map(Quantity::quantity).sum()
    }

    fn total_of(&self, item_code: &str) -> u32 {
        self.content()
            .iter()
            .find(|i| i.code() == item_code)
            .map_or(0, Quantity::quantity)
    }

    fn contains_all(&self, items: &[impl Code + Quantity]) -> bool {
        items
            .iter()
            .all(|i| self.total_of(i.code()) >= i.quantity())
    }
}

pub trait LimitedContainer: ItemContainer {
    fn is_full(&self) -> bool;
    fn has_room_for_all(&self, items: &[impl Code + Quantity]) -> bool;
    fn has_room_for_drops_from(&self, entity: &impl HasDropTable) -> bool;

    fn has_room_for(&self, item: impl Code + Quantity) -> bool {
        self.has_room_for_all(&[item])
    }
}

pub trait SlotLimited: LimitedContainer {
    fn free_slots(&self) -> u32;
}

pub trait SpaceLimited: LimitedContainer {
    fn max_items(&self) -> u32;

    fn free_space(&self) -> u32 {
        self.max_items().saturating_sub(self.total_items())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{inventory_client, inventory_with};
    use openapi::models::{DropRateSchema, InventorySlotSchema};

    fn filled_inventory() -> Vec<InventorySlotSchema> {
        inventory_with(&[
            ("a", 5),
            ("b", 5),
            ("c", 5),
            ("d", 5),
            ("e", 5),
            ("f", 5),
            ("g", 5),
            ("h", 5),
            ("i", 5),
            ("j", 5),
            ("k", 5),
            ("l", 5),
            ("m", 5),
            ("n", 5),
            ("o", 5),
            ("p", 5),
            ("q", 5),
            ("r", 5),
            ("s", 5),
            ("t", 5),
        ])
    }

    #[test]
    fn total_items() {
        // empty
        let inv = inventory_client(inventory_with(&[]), 100);
        assert_eq!(inv.total_items(), 0);

        // with items
        let inv = inventory_client(inventory_with(&[("ore", 10), ("wood", 5), ("ore", 3)]), 100);
        assert_eq!(inv.total_items(), 18);
    }

    #[test]
    fn total_of() {
        // item present
        let inv = inventory_client(inventory_with(&[("ore", 10), ("wood", 5)]), 100);
        assert_eq!(inv.total_of("ore"), 10);

        // item missing
        let inv = inventory_client(inventory_with(&[("ore", 10)]), 100);
        assert_eq!(inv.total_of("wood"), 0);

        // empty inventory
        let inv = inventory_client(inventory_with(&[]), 100);
        assert_eq!(inv.total_of("ore"), 0);
    }

    #[test]
    fn contains_all() {
        // sufficient
        let inv = inventory_client(inventory_with(&[("ore", 10), ("wood", 5)]), 100);
        assert!(inv.contains_all(&[("ore", 10), ("wood", 5)]));

        // insufficient quantity
        let inv = inventory_client(inventory_with(&[("ore", 3)]), 100);
        assert!(!inv.contains_all(&[("ore", 5)]));

        // missing item
        let inv = inventory_client(inventory_with(&[("ore", 10)]), 100);
        assert!(!inv.contains_all(&[("ore", 10), ("wood", 1)]));

        // empty list
        let inv = inventory_client(inventory_with(&[]), 100);
        let empty: [(String, u32); 0] = [];
        assert!(inv.contains_all(&empty));
    }

    #[test]
    fn free_slots() {
        // all empty
        let inv = inventory_client(inventory_with(&[]), 100);
        assert_eq!(inv.free_slots(), 20);

        // partial
        let inv = inventory_client(inventory_with(&[("ore", 10)]), 100);
        assert_eq!(inv.free_slots(), 19);

        // none free
        let inv = inventory_client(filled_inventory(), 100);
        assert_eq!(inv.free_slots(), 0);
    }

    #[test]
    fn free_space() {
        // empty
        let inv = inventory_client(inventory_with(&[]), 100);
        assert_eq!(inv.free_space(), 100);

        // partial
        let inv = inventory_client(inventory_with(&[("ore", 30)]), 100);
        assert_eq!(inv.free_space(), 70);

        // full
        let inv = inventory_client(inventory_with(&[("ore", 100)]), 100);
        assert_eq!(inv.free_space(), 0);
    }

    #[test]
    fn is_full() {
        // no items and no free slots
        let inv = inventory_client(filled_inventory(), 100);
        assert!(inv.is_full());

        // with free slots
        let inv = inventory_client(inventory_with(&[("ore", 5)]), 100);
        assert!(!inv.is_full());

        // items below max
        let inv = inventory_client(inventory_with(&[("ore", 50)]), 100);
        assert!(!inv.is_full());
    }

    #[test]
    fn has_room_for_all() {
        // with space
        let inv = inventory_client(inventory_with(&[]), 100);
        assert!(inv.has_room_for_all(&[("ore", 10)]));

        // no free slots
        let inv = inventory_client(filled_inventory(), 100);
        assert!(!inv.has_room_for_all(&[("iron", 10)]));

        // not enough space
        let inv = inventory_client(inventory_with(&[("ore", 95)]), 100);
        assert!(!inv.has_room_for_all(&[("wood", 10)]));

        // existing item, no slot needed
        let inv = inventory_client(inventory_with(&[("ore", 5)]), 100);
        assert!(inv.has_room_for_all(&[("ore", 10)]));

        // multiple items
        let inv = inventory_client(inventory_with(&[]), 100);
        assert!(inv.has_room_for_all(&[("ore", 10), ("wood", 20)]));

        // multiple items, one fails
        let inv = inventory_client(inventory_with(&[("ore", 90)]), 100);
        assert!(!inv.has_room_for_all(&[("ore", 5), ("wood", 10)]));
    }

    #[test]
    fn has_room_for() {
        // delegates to has_room_for_all
        let inv = inventory_client(inventory_with(&[]), 100);
        assert!(inv.has_room_for(("ore", 10)));

        // no space
        let inv = inventory_client(inventory_with(&[("ore", 100)]), 100);
        assert!(!inv.has_room_for(("wood", 1)));
    }

    #[test]
    fn has_room_for_drops() {
        // empty inventory
        let inv = inventory_client(inventory_with(&[]), 100);
        let drops = vec![DropRateSchema::new("ore".into(), 100, 1, 3)];
        assert!(inv.has_room_for_drops_from(&drops));

        // no free slots
        let inv = inventory_client(filled_inventory(), 100);
        let drops = vec![DropRateSchema::new("iron".into(), 100, 1, 1)];
        assert!(!inv.has_room_for_drops_from(&drops));

        // not enough space
        let inv = inventory_client(inventory_with(&[("ore", 99)]), 100);
        let drops = vec![DropRateSchema::new("iron".into(), 100, 5, 10)];
        assert!(!inv.has_room_for_drops_from(&drops));
    }
}
