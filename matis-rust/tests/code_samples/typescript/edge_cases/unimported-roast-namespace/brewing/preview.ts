// A supplier namespace without an import is not a project-wide lookup.
export function preview(): string {
  return Unimported.reserve("espresso");
}
