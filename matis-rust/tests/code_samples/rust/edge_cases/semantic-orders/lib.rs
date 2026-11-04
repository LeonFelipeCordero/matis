pub mod brewing;
pub mod reporting;
pub mod receipt;

pub fn order_receipt(names: &[&str]) -> String {
    crate::receipt::render(names)
}
