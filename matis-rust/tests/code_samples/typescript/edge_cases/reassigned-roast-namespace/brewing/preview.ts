import * as roast from "../inventory/roast.ts";

// Deliberately unsafe import mutation: the resolver must not retain its identity.
export function preview(supplier: { reserve(name: string): string }): string {
  roast = supplier;
  return roast.reserve("espresso");
}
