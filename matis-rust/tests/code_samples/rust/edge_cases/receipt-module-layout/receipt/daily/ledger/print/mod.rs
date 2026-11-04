use crate::reporting::report;

pub fn render(names: &[&str]) -> String {
    let receipt = report::render(names);
    format!("Daily ledger: {}", receipt)
}
