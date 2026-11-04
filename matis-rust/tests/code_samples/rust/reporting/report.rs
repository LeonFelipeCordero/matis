use self::render as render_receipt;
use crate::brewing::recipe;
use crate::brewing::recipe::Recipe as Drink;

pub struct Report {
    names: Vec<String>,
}

impl Report {
    pub fn new(names: &[&str]) -> Self {
        Self {
            names: names.iter().map(|name| name.to_string()).collect(),
        }
    }

    pub fn render(&self) -> String {
        let names: Vec<&str> = self.names.iter().map(|name| name.as_str()).collect();
        render_receipt(&names)
    }

    pub fn estimate(&self, cups: i32) -> i32 {
        let drink: Drink = Drink::new("filter", cups);
        drink.portions()
    }
}

pub fn total(names: &[&str]) -> i32 {
    names.iter().map(|name| recipe::portions(name)).sum()
}

pub fn render(names: &[&str]) -> String {
    let entries: Vec<String> = names
        .iter()
        .map(|name| format!("{}={}", name, recipe::portions(name)))
        .collect();
    format!("coffee:{};total={}", entries.join(","), total(names))
}
