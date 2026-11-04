import { Recipe } from "../../../brewing/Recipe.js";

export default class HouseRecipe {
  static portions(name) { return Recipe.portions(name); }
}
