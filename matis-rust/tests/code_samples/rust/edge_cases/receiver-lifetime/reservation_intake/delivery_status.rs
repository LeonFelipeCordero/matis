impl super::records::DeliveryStatus {
    pub fn commit(&self, order: &str) -> String {
        format!("delivery-status:{}", order)
    }

    pub fn fulfill(&self, order: &str) -> String {
        self.commit(order)
    }
}
