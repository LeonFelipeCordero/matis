mod records;
mod archived;
mod vendor;
mod delivery;
mod delivery_status;

use crate::reservation_intake::records::ReservationLedger;
use crate::reservation_intake::records::PickupStatus;
use crate::reservation_intake::records::AuditLedger;
use crate::reservation_intake::records::ReconciledLedger;

impl ReservationLedger {
    pub fn commit(&self, order: &str) -> String {
        format!("pickup:{}:{}", self.orders.len(), order)
    }

    pub fn fulfill(&self, order: &str) -> String {
        self.commit(order)
    }
}

mod archive_writer {
    use crate::reservation_intake::records::ArchiveLedger;

    impl ArchiveLedger {
        pub fn commit(&self, order: &str) -> String {
            format!("archive:{}:{}", self.directory, order)
        }
    }
}

impl PickupStatus {
    pub fn commit(&self, order: &str) -> String {
        format!("pickup-status:{}", order)
    }

    pub fn fulfill(&self, order: &str) -> String {
        self.commit(order)
    }
}

impl AuditLedger {
    pub fn commit(&self, order: &str) -> String {
        format!("recovery:{}:{}", self.journal.len(), order)
    }
}

trait Reconcile {
    fn commit(&self, order: &str) -> String;
}

impl Reconcile for ReconciledLedger {
    fn commit(&self, order: &str) -> String {
        format!("reconciled:{}:{}", self.journal.len(), order)
    }
}

struct PickupStation {
    ledger: ReservationLedger,
}

impl PickupStation {
    fn fulfill(&self, order: &str) -> String {
        self.ledger.commit(order)
    }
}

pub fn open_pickup(order: &str) -> String {
    let ledger = ReservationLedger { orders: Vec::new() };
    receive(order, &ledger)
}

fn receive(order: &str, parameter: &ReservationLedger) -> String {
    let local: ReservationLedger = ReservationLedger { orders: Vec::new() };
    parameter.commit(order);
    local.commit(order)
}

fn receive_qualified(order: &str, ledger: &records::ReservationLedger) -> String {
    ledger.commit(order)
}

fn deliver(order: &str, ledger: &records::DeliveryLedger) -> String {
    ledger.commit(order)
}

fn archive(order: &str, ledger: &records::ArchiveLedger) -> String {
    ledger.commit(order)
}

fn receive_unknown(order: &str, unknown: MissingProvider) -> String {
    unknown.commit(order)
}

fn restore(order: &str, ledger: MissingProvider) -> String {
    ledger.commit(order)
}

fn adjust(order: &str, mut ledger: ReservationLedger, replacement: ReservationLedger) -> String {
    ledger += replacement;
    ledger.commit(order)
}

fn read_archived(order: &str, ledger: &archived::ReservationLedger) -> String {
    ledger.commit(order)
}

fn audit(order: &str, ledger: &AuditLedger) -> String {
    ledger.commit(order)
}

fn reconcile(order: &str, ledger: &ReconciledLedger) -> String {
    ledger.commit(order)
}
