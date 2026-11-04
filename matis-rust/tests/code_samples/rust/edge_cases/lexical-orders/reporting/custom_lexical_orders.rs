use crate::lexical_orders::brew;

// Custom shop macros need their own file: conservative extraction scans all definitions.
pub fn source_label(names: &[&str]) {
    macro_rules! println {
        ($($payload:tt)*) => { stringify!($($payload)*) };
    }
    dbg!(println!("{}", brew(names)));
}
