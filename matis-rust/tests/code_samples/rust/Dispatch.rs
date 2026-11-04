#[path = "reporting/dispatch_format.rs"]
mod dispatch_report;

use crate::brewing::recipe;

mod dispatch_labels {
    pub fn pickup(name: &str) -> String {
        format!("pickup:{}", name)
    }
}

pub struct Dispatch {
    minimum: i32,
}

impl Dispatch {
    pub fn new(minimum: i32) -> Self {
        Self { minimum }
    }

    pub fn receipts(&self, names: &[&str]) -> Vec<String> {
        names.iter().filter(|name| recipe::portions(name) >= self.minimum).map(|name| {
            fn label(name: &str) -> String {
                dispatch_labels::pickup(name)
            }
            dispatch_report::encode(&label(name), &recipe::preview(name))
        }).collect()
    }

    pub fn deliver(&self, names: &[&str], send: fn(String)) {
        for receipt in self.receipts(names) {
            println!("{}", dispatch_labels::pickup(&receipt));
            send(receipt);
        }
    }
}
