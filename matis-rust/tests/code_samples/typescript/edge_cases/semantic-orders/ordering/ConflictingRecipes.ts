// Conflicting supplier bindings are a menu configuration error, not a house recipe.
import { Recipe as Drink } from "../../../brewing/Recipe.ts";
import Drink from "../reporting/HouseRecipe.ts";

export function portions(name: string) {
  return Drink.portions(name);
}
