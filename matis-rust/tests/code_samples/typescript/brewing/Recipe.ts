import { Report } from "../reporting/Report.ts";

export class Recipe {
  static portions(name: string): number {
    switch (name) {
      case "espresso": return 2;
      case "filter": return 3;
      default: return 0;
    }
  }

  static price(name: string): number {
    const prices: Record<string, number> = { espresso: 2.5, filter: 3 };
    return prices[name] ?? 0;
  }

  static dose(name: string): number {
    const doses: Record<string, number> = { espresso: 18, filter: 20 };
    return doses[name] ?? 0;
  }

  describe(name: string): string {
    return `${name}: ${Recipe.dose(name)}g beans`;
  }

  static preview(name: string): string {
    return Report.render([name]);
  }
}
