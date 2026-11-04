import * as roast from "../inventory/roast.ts";

export function preview(roast: { reserve(name: string): string }): string {
  return roast.reserve("espresso");
}

export function previewFilter() {
  return roast.reserve("filter");
}
