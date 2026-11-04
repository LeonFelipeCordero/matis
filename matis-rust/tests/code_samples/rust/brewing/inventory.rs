use super::recipe;
use std::collections::BTreeMap;

type Adjustment = fn(i32) -> i32;
const KEEP_NONNEGATIVE: Adjustment = |count| count.max(0);

pub struct Inventory {
    beans: BTreeMap<String, i32>,
}

impl Inventory {
    pub fn new(stock: &[(&str, i32)]) -> Self {
        let beans = stock
            .iter()
            .map(|(name, count)| (name.to_string(), *count))
            .collect();
        Self { beans }
    }

    pub fn remaining(&self, name: &str) -> i32 {
        self.beans.get(name).copied().unwrap_or(0)
    }

    pub fn reserve(&mut self, name: &str) -> bool {
        let dose = recipe::portions(name);
        if dose == 0 || self.remaining(name) < dose {
            return false;
        }
        *self.beans.entry(name.to_string()).or_default() -= dose;
        true
    }

    pub fn restock(&mut self, deliveries: &[(&str, i32)], adjust: Adjustment) {
        deliveries.iter().for_each(|(name, count)| {
            fn adjusted_units(value: i32, adjust: Adjustment) -> i32 {
                KEEP_NONNEGATIVE(adjust(value))
            }
            let normalize = |value: i32| adjusted_units(value, adjust);
            *self.beans.entry(name.to_string()).or_default() += normalize(*count);
        });
    }
}
