import { Recipe } from "../brewing/Recipe.ts";

export class Report {
  static total(names: Array<string>): number {
    return names.reduce((sum, name) => sum + Recipe.portions(name), 0);
  }

  static render(names: Array<string>): string {
    const entries = names.map(name => `${name}=${Recipe.portions(name)}`);
    return `coffee:${entries.join(",")};total=${Report.total(names)}`;
  }
}
