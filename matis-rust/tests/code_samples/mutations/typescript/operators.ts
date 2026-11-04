export function coffeeOperators(a: number, b: number, ready: boolean, stocked: boolean): number {
  const added = a + b;
  const subtracted = a - b;
  const multiplied = a * b;
  const divided = a / b;
  const equal = a == b;
  const unequal = a != b;
  const less = a < b;
  const lessEqual = a <= b;
  const greater = a > b;
  const greaterEqual = a >= b;
  const strictEqual = a === b;
  const strictUnequal = a !== b;
  const both = ready && stocked;
  const either = ready || stocked;
  a += b;
  a -= b;
  a *= b;
  a /= b;
  const unaryPositive = +a;
  const unaryNegative = -b;
  return divided;
}
