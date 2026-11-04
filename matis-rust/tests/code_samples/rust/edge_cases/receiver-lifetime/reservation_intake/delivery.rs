impl super::records::DeliveryLedger {
    pub fn commit(&self, order: &str) -> String {
        format!("delivery:{}:{}", self.address, order)
    }

    pub fn fulfill(&self, order: &str) -> String {
        self.commit(order)
    }
}
