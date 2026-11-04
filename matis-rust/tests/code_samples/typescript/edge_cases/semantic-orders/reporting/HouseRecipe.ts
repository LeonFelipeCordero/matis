import { Recipe } from "../../../brewing/Recipe.ts";

export default class HouseRecipe {
  static portions(name: string): number { return Recipe.portions(name); }
}
