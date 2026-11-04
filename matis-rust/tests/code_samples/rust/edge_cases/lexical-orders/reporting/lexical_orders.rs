use crate::reporting::report;
use crate::brewing::recipe;
use seasonal_supplier::SeasonalRecipe;

// Café receipts preserve UTF-8 offsets inside expanded standard macro payloads.
pub fn brew(names: &[&str]) -> String {
    report::render(names)
}

macro_rules! custom {
    ($($payload:tt)*) => { stringify!($($payload)*) };
}

pub fn source_labels(names: &[&str]) {
    println!("{}", stringify!(brew(names)));
    println!("{}", custom!(brew(names)));
}

pub fn original_span(names: &[&str]) {
    println!("café: {}", brew(names));
}

pub fn local_binding(names: &[&str]) {
    println!("{}", {
        let brew = |drinks| report::render(drinks);
        brew(names)
    });
}

pub fn closure_parameter(names: &[&str], recipes: &[fn(&[&str]) -> String]) {
    recipes.iter().for_each(|brew| { println!("{}", brew(names)); });
    println!("{}", brew(names));
}

pub fn method_receiver(recipe: &dyn SeasonalRecipe, name: &str) {
    println!("{}", recipe.portions(name));
}

pub fn nested_binding(names: &[&str]) {
    println!("{}", {
        let brew = |drinks| report::render(drinks);
        dbg!(brew(names))
    });
}

pub fn nested_mutable_binding(names: &[&str]) {
    println!("{}", {
        let mut brew = |drinks| report::render(drinks);
        dbg!(brew(names))
    });
}

pub fn nested_function(names: &[&str]) {
    println!("{}", {
        fn brew(drinks: &[&str]) -> String { report::render(drinks) }
        dbg!(brew(names))
    });
}

pub fn nested_callback(names: &[&str], brew: fn(&[&str]) -> String) {
    println!("{}", { dbg!(brew(names)) });
}

pub fn nested_custom_definition(names: &[&str]) {
    println!("{}", {
        macro_rules! dbg {
            ($($payload:tt)*) => { stringify!($($payload)*) };
        }
        dbg!(brew(names))
    });
}

pub fn aliased_standard_namespace(names: &[&str]) {
    use seasonal_macros as std;
    println!("{}", std::dbg!(brew(names)));
}
