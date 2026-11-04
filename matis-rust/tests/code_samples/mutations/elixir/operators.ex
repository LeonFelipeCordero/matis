defmodule CoffeeOperators do
  def brew(a, b, ready, stocked) do
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
    strict_equal = a === b
    strict_unequal = a !== b
    both = ready and stocked
    either = ready or stocked
    truthy_both = ready && stocked
    truthy_either = ready || stocked
    unary_positive = +a
    unary_negative = -b
    divided
  end
end
