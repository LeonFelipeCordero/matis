import { Recipe as Drink } from "../../../brewing/Recipe.js";
import DefaultDrink from "../reporting/HouseRecipe.js";

export function house(name) {
  const note = "Drink = seasonal";
  return { note, portions: Drink.portions(name), defaultPortions: DefaultDrink.portions(name) };
}

export function replaceRecipe(name, replacement) {
  Drink = replacement;
  return Drink.portions(name);
}

export function parameterRecipe(name, Drink) {
  return Drink.portions(name);
}

export function unpackedRecipe(name, recipes) {
  const { Drink } = recipes;
  return Drink.portions(name);
}

export function suppliedRecipe(name, factory, object) {
  return [factory().portions(name), object.portions(name)];
}

export function unknownBrewer(names, unknown, factory) {
  return [unknown.brew(names), factory().brew(names)];
}
