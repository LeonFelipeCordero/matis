use crate::reporting::report;

pub mod print;
pub mod daily {
    pub mod ledger {
        pub mod print;
    }
}

#[path = "special/receipt_printer.rs"]
#[allow(dead_code)]
// Receipt printers can be supplied by the shop's reporting adapter.
pub mod custom;

pub mod seasonal {
    #[path = "../../special/receipt_printer.rs"]
    pub mod print;
}

#[path = "receipt/alternate"]
pub mod alternate {
    pub mod print;
}

pub fn render(names: &[&str]) -> String {
    report::render(names)
}

fn audit(names: &[&str]) -> usize {
    names.len()
}

pub fn close(names: &[&str]) {
    println!("{}", print::render(names));
    println!("{}", self::print::render(names));
    println!("{}", super::layout_receipt::print::render(names));
    println!("{}", crate::print::render(names));
    println!("{}", custom::render(names));
    println!("{}", seasonal::print::render(names));
    println!("{}", alternate::print::render(names));
    println!("{}", daily::ledger::print::render(names));
    println!("{}", audit(names));
}
