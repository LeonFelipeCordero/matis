class CoffeeOperators {
    double brew(double a, double b, boolean ready, boolean stocked) {
        double added = a + b;
        double subtracted = a - b;
        double multiplied = a * b;
        double divided = a / b;
        boolean equal = a == b;
        boolean unequal = a != b;
        boolean less = a < b;
        boolean lessEqual = a <= b;
        boolean greater = a > b;
        boolean greaterEqual = a >= b;
        boolean both = ready && stocked;
        boolean either = ready || stocked;
        a += b;
        a -= b;
        a *= b;
        a /= b;
        double unaryPositive = +a;
        double unaryNegative = -b;
        return divided;
    }
}
