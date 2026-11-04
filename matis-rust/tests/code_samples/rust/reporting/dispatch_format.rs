/// Combine the pickup label and brewing receipt for a delivery notification.
pub fn encode(label: &str, receipt: &str) -> String {
    format!("{}\n{}", label, receipt)
}
