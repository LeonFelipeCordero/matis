import { greet } from "../services/greeter.js";
import { greet as sourceGreet } from "../services/greeter.ts";
import type { Greeting } from "../services/greeter.js";
import { badge } from "../views/Badge.jsx";
import { badge as sourceBadge } from "../views/Badge.tsx";
import { suffix } from "../legacy/helpers.js";

export function run(): Greeting {
  badge();
  sourceBadge();
  return { message: greet() + sourceGreet() + suffix() };
}
