import { Recipe } from "../brewing/Recipe.js";

export class Fulfillment {
  route(active, amount) {
    if (active) {
      if (amount > 8) return "courier";
      else if (amount > 3) return "table";
      else if (amount > 0) return "counter";
    }
    return "hold";
  }

  quote(name) {
    return Recipe.portions(name);
  }
}
