pub mod report;

// Closing reports use a nested module, separate from per-order receipts.
pub mod daily {
    use super::report::Report as Receipt;
    pub mod totals;

    pub fn close(names: &[&str]) -> String {
        let receipt: Receipt = Receipt::new(names);
        let rendered = receipt.render();
        let estimated = receipt.estimate(names.len() as i32);
        format!(
            "{};daily={};estimate={}",
            rendered,
            totals::portions(names),
            estimated
        )
    }
}
