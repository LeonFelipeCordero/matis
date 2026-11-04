use self::brew as render_order;
use super::inventory::Inventory;
use super::recipe;
use crate::reporting::report;

pub struct Brewer {
    stock: Inventory,
}

impl Brewer {
    pub fn new(stock: Inventory) -> Self {
        Self { stock }
    }

    pub fn brew(&mut self, names: &[&str]) -> String {
        let mut available = Vec::new();
        for name in names {
            if self.stock.reserve(name) {
                available.push(*name);
            }
        }
        render_order(&available)
    }

    pub fn remaining(&self, name: &str) -> i32 {
        self.stock.remaining(name)
    }
}

pub fn brew(names: &[&str]) -> String {
    if names.is_empty() {
        return report::render(&[]);
    }
    let valid: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| recipe::portions(name) > 0)
        .collect();
    report::render(&valid)
}

pub fn retry(attempts: i32) -> i32 {
    if attempts <= 0 {
        return 0;
    }
    1 + retry(attempts - 1)
}

pub fn score(batches: &[Vec<&str>]) -> i32 {
    let mut result = 0;
    for batch in batches {
        for name in batch {
            let count = recipe::portions(name);
            if count > 0 && count <= 3 {
                result += count;
            }
        }
    }
    result
}

pub fn classify(amount: i32) -> i32 {
    match amount {
        i32::MIN..=0 => 0,
        1..=3 => 1,
        4..=8 => 2,
        _ => 3,
    }
}

pub fn ready(water: i32, beans: i32, cups: i32, temperature: i32) -> bool {
    water > 0 && beans > 0 && cups > 0 && temperature >= 90 && temperature <= 96
}

// Batch water in milliliters, corrected for brewing temperature and grind size.
pub fn calibrate(
    beans: i32,
    ratio: i32,
    water_offset: i32,
    batches: i32,
    temperature: i32,
    target_temperature: i32,
    grind: i32,
    target_grind: i32,
) -> i32 {
    (beans * ratio + water_offset) * batches
        + (temperature - target_temperature) * 2
        + (grind - target_grind) * 5
}
