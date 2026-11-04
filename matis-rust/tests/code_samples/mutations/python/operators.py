def coffee_operators(a, b, ready, stocked):
    added = a + b
    subtracted = a - b
    multiplied = a * b
    divided = a / b
    equal = a == b
    unequal = a != b
    less = a < b
    less_equal = a <= b
    greater = a > b
    greater_equal = a >= b
    both = ready and stocked
    either = ready or stocked
    a += b
    a -= b
    a *= b
    a /= b
    unary_positive = +a
    unary_negative = -b
    return divided
