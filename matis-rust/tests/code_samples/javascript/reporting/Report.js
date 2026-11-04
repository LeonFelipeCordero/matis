import { Recipe } from "../brewing/Recipe.js";

export class Report {
  static total(names) {
    return names.reduce((sum, name) => sum + Recipe.portions(name), 0);
  }

  static render(names) {
    const entries = names.map(name => `${name}=${Recipe.portions(name)}`);
    return `coffee:${entries.join(",")};total=${Report.total(names)}`;
  }
}
