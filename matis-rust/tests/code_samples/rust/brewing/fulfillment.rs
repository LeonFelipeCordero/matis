use crate::brewing::recipe;

pub fn route(active: bool, amount: i32) -> &'static str {
    if active {
        if amount > 8 {
            return "courier";
        } else if amount > 3 {
            return "table";
        } else if amount > 0 {
            return "counter";
        }
    }
    "hold"
}

pub fn quote(name: &str) -> i32 {
    recipe::portions(name)
}
