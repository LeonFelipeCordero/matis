pub fn coffee_operators(mut a: f64, b: f64, ready: bool, stocked: bool) -> f64 {
    let added = a + b;
    let subtracted = a - b;
    let multiplied = a * b;
    let divided = a / b;
    let equal = a == b;
    let unequal = a != b;
    let less = a < b;
    let less_equal = a <= b;
    let greater = a > b;
    let greater_equal = a >= b;
    let both = ready && stocked;
    let either = ready || stocked;
    a += b;
    a -= b;
    a *= b;
    a /= b;
    let unary_negative = -b;
    divided
}
