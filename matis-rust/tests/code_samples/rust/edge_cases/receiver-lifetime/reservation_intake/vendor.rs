pub struct ReservationLedger {
    pub endpoint: String,
}

impl ReservationLedger {
    pub fn commit(&self, order: &str) -> String {
        format!("{}:{}", self.endpoint, order)
    }
}
