package coffee

func coffeeOperators(a, b float64, ready, stocked bool) float64 {
    added := a + b
    subtracted := a - b
    multiplied := a * b
    divided := a / b
    equal := a == b
    unequal := a != b
    less := a < b
    lessEqual := a <= b
    greater := a > b
    greaterEqual := a >= b
    both := ready && stocked
    either := ready || stocked
    a += b
    a -= b
    a *= b
    a /= b
    unaryPositive := +a
    unaryNegative := -b
    return divided
}
