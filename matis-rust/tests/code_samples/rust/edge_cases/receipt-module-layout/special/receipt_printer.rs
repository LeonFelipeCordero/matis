use crate::reporting::report;

pub fn render(names: &[&str]) -> String {
    format!("Seasonal receipt: {}", report::render(names))
}
