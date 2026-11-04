pub mod daily;

pub fn render(names: &[&str]) -> String {
    names.join(" / ")
}
