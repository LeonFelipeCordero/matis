pub struct Order {
    pub customer: String,
    pub names: Vec<String>,
    pub total: i32,
    pub status: &'static str,
}

#[derive(Default)]
pub struct PickupQueue { pub orders: Vec<Order> }

impl PickupQueue {
    pub fn record(&mut self, mut order: Order, fee: i32) -> String {
        order.total = tracking_quote(&order, 0, 0, fee, 0, 0, 0);
        order.status = "ready";
        let label = tracking_formatter()((&order, "counter"), "ready");
        self.orders.push(order);
        label
    }
    pub fn take(self: Box<Self>, status: &str) -> (Vec<Order>, Self) {
        let (ready, pending) = self.orders.into_iter().partition(|order| order.status == status);
        (ready, Self { orders: pending })
    }
}

pub fn tracking_quote(order: &Order, tax_percent: i32, tip: i32,
                      fee: i32, discount_percent: i32, deposit: i32, credit: i32) -> i32 {
    let discounted = order.total * (100 - discount_percent) / 100;
    discounted + discounted * tax_percent / 100 + tip + fee - deposit - credit
}

pub fn tracking_formatter() -> impl Fn((&Order, &str), &str) -> String {
    |(order, counter): (&Order, &str), status: &str| format!("{}: {} drinks {} at {}", order.customer, order.names.len(), status, counter)
}
