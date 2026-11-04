struct LiteralLedger { count: usize }
struct OtherLedger { count: usize }

impl LiteralLedger {
    fn new() -> Self { Self { count: 0 } }
    fn record(&self) {}
    fn flush(&mut self) {}
}

impl OtherLedger {
    fn record(&self) {}
}

trait LedgerAudit { fn audit(&self); }
impl LedgerAudit for LiteralLedger { fn audit(&self) {} }

fn direct_ledger() {
    let ledger = LiteralLedger { count: 0 };
    ledger.record();
}

fn mutable_ledger() {
    let mut ledger = LiteralLedger { count: 0 };
    ledger.flush();
}

fn other_ledger() {
    let ledger = OtherLedger { count: 0 };
    ledger.record();
}

fn associated_factory() {
    let ledger = LiteralLedger::new();
    ledger.record();
}

fn reassigned_ledger() {
    let mut ledger = LiteralLedger { count: 0 };
    ledger = LiteralLedger::new();
    ledger.record();
}

fn shadowed_ledger() {
    let ledger = LiteralLedger { count: 0 };
    {
        let ledger = LiteralLedger::new();
        ledger.record();
    }
    ledger.record();
}

fn trait_ledger() {
    let ledger = LiteralLedger { count: 0 };
    ledger.audit();
}

fn associated_instance_method() {
    LiteralLedger::record(&LiteralLedger { count: 0 });
}

fn destructured_ledger() {
    let (ledger, _) = (LiteralLedger { count: 0 }, 0);
    ledger.record();
}

impl LiteralLedger {
    fn consume(self: Box<Self>) { self.record(); }
}

fn associated_typed_receiver(ledger: Box<LiteralLedger>) {
    LiteralLedger::consume(ledger);
}
