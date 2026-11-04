use crate::reporting::report;

pub fn render(names: &[&str]) -> String {
    format!("Takeaway receipt: {}", report::render(names))
}
