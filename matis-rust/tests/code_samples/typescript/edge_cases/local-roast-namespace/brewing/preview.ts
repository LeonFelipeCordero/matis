import * as roast from "../inventory/roast.ts";

export function preview(): string {
  const roast = {
    reserve(name: string): string {
      return `Decaf sample: ${name}, 18g`;
    },
  };
  return roast.reserve("espresso");
}

export function previewFilter() {
  return roast.reserve("filter");
}
