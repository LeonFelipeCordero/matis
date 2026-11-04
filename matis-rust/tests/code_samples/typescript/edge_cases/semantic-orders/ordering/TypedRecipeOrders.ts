import type { Recipe as Drink } from "../../../brewing/Recipe.ts";

export function portions(name: string) {
  return Drink.portions(name);
}
