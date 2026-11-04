pub fn delivery_label(names: &[&str], customer: &str) -> String {
    format!("Deliver {} drinks to {}", names.len(), customer)
}

pub fn manifest(orders: &[(&str, Vec<&str>)]) -> Vec<String> {
    orders
        .iter()
        .map(|(customer, names)| delivery_label(names, customer))
        .collect()
}
