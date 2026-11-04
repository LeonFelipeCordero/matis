use self::portions as portion_count;
use crate::reporting::report;

pub struct Recipe {
    pub name: String,
    pub cups: i32,
}

impl Recipe {
    pub fn new(name: &str, cups: i32) -> Self {
        Self {
            name: name.trim().to_lowercase(),
            cups,
        }
    }

    pub fn portions(&self) -> i32 {
        portion_count(&self.name) * self.cups
    }

    pub fn scale(self, cups: i32) -> Self {
        Self { cups, ..self }
    }

    pub fn label(&self, (prefix, suffix): (&str, &str)) -> String {
        format!("{}{}{}", prefix, self.name, suffix)
    }
}

pub fn portions(name: &str) -> i32 {
    match name {
        "espresso" => 2,
        "filter" => 3,
        _ => 0,
    }
}

pub fn preview(name: &str) -> String {
    report::render(&[name])
}
