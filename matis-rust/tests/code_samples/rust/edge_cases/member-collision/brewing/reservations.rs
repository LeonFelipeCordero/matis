use super::inventory::Inventory;

pub fn reserve(names: &[&str], customer: &str) -> String {
    format!("Reserved for {}: {}", customer, names.join(", "))
}

pub fn queue_order(inventory: &mut Inventory, names: &[&str], customer: &str) -> String {
    for name in names {
        if !inventory.reserve(name) {
            return format!("Waiting for stock: {}", customer);
        }
    }
    reserve(names, customer)
}

pub fn request_supplier<S>(supplier: &mut S, names: &[&str], customer: &str) -> String {
    if !supplier.reserve(names) {
        return format!("Supplier cannot fill: {}", customer);
    }
    supplier.delivery_label(names, customer)
}
