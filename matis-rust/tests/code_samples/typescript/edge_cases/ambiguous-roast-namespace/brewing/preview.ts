// Two suppliers expose the same type, but neither is imported here.
export function preview(): string {
  return Ambiguous.reserve("espresso");
}
