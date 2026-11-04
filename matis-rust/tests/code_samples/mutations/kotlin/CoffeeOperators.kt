fun coffeeOperators(input: Double, b: Double, ready: Boolean, stocked: Boolean): Double {
    var a = input
    val added = a + b
    val subtracted = a - b
    val multiplied = a * b
    val divided = a / b
    val equal = a == b
    val unequal = a != b
    val less = a < b
    val lessEqual = a <= b
    val greater = a > b
    val greaterEqual = a >= b
    val both = ready && stocked
    val either = ready || stocked
    a += b
    a -= b
    a *= b
    a /= b
    val unaryPositive = +a
    val unaryNegative = -b
    return divided
}
