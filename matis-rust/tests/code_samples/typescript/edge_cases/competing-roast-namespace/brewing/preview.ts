// Deliberately conflicting bindings: neither imported supplier may be guessed.
import * as roast from "../inventory/roast.ts";
import * as roast from "../inventory/decaf.ts";

export function preview() {
  return roast.reserve("espresso");
}
