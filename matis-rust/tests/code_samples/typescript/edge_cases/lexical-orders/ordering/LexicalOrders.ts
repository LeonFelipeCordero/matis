import { Recipe as Drink } from "../../../brewing/Recipe.ts";

export function single(recipes, name: string) {
  const counts = recipes.map((Drink: unknown) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function parenthesized(recipes, name) {
  const counts = recipes.map((Drink) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function functionParameter(recipes, name) {
  const counts = recipes.map(function(Drink) { return Drink.portions(name); });
  return { counts, house: Drink.portions(name) };
}

export function namedFunction(recipes, name) {
  const counts = recipes.map(function inspect(Drink) { return Drink.portions(name); });
  return { counts, house: Drink.portions(name) };
}

export function objectParameter(recipes, name) {
  const counts = recipes.map(({Drink}) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function arrayParameter(recipes, name) {
  const counts = recipes.map(([Drink]) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function restParameter(recipes, name) {
  const counts = recipes.map((...Drink) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function nestedParameter(recipes, name) {
  const counts = recipes.map(({seasonal: {Drink}}) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function renamedParameter(recipes, name) {
  const counts = recipes.map(({recipe: Drink}) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function defaultParameter(recipes, name) {
  const counts = recipes.map((Drink = recipes[0]) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function restObject(recipes, name) {
  const counts = recipes.map(({label, ...Drink}) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function defaultObject(recipes, name) {
  const counts = recipes.map(({recipe: Drink = recipes[0]}) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function selfNamed(recipes, name) {
  const counts = recipes.map(function Drink() { return Drink.portions(name); });
  return { counts, house: Drink.portions(name) };
}

export function localCallable(recipes, name) {
  const counts = recipes.map(() => {
    function Drink(names) { return names.join(", "); }
    return Drink.portions(name);
  });
  return { counts, house: Drink.portions(name) };
}

export function propertyKey(recipes, name) {
  const counts = recipes.map(({Drink: seasonal}) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}

export function defaultValue(recipes, name) {
  const counts = recipes.map((seasonal = Drink) => Drink.portions(name));
  return { counts, house: Drink.portions(name) };
}
