pub struct ReservationLedger {
    pub orders: Vec<String>,
}

pub struct DeliveryLedger {
    pub address: String,
}

pub struct ArchiveLedger {
    pub directory: String,
}

pub enum PickupStatus {
    Waiting(String),
    Collected(String),
}

pub enum DeliveryStatus {
    Scheduled(String),
    Delivered(String),
}

pub struct AuditLedger {
    pub journal: Vec<String>,
}

impl AuditLedger {
    pub fn commit(&self, order: &str) -> String {
        format!("audit:{}:{}", self.journal.len(), order)
    }
}

pub struct ReconciledLedger {
    pub journal: Vec<String>,
}
