import { Recipe as Drink } from "../../../brewing/Recipe.ts";
import DefaultDrink from "../reporting/HouseRecipe.ts";

export function house(name: string) {
  const note = "Drink = seasonal";
  return { note, portions: Drink.portions(name), defaultPortions: DefaultDrink.portions(name) };
}

export function replaceRecipe(name: string, replacement) {
  Drink = replacement;
  return Drink.portions(name);
}

export function parameterRecipe(name: string, Drink) {
  return Drink.portions(name);
}

export function unpackedRecipe(name: string, recipes) {
  const { Drink } = recipes;
  return Drink.portions(name);
}

export function suppliedRecipe(name: string, factory, object) {
  return [factory().portions(name), object.portions(name)];
}

export function unknownBrewer(names: string[], unknown, factory) {
  return [unknown.brew(names), factory().brew(names)];
}
