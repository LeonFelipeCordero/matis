// café: receipt output uses byte offsets
pub fn tasting() -> i32 { 2 }
pub fn print_receipt() {
    println!("{}", format!("{}", tasting()));
    receipt! { drink => missing() }
}
