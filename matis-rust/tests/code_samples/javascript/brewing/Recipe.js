import { Report } from "../reporting/Report.js";

export class Recipe {
  static portions(name) {
    switch (name) {
      case "espresso": return 2;
      case "filter": return 3;
      default: return 0;
    }
  }

  static price(name) {
    return { espresso: 2.5, filter: 3 }[name] ?? 0;
  }

  static dose(name) {
    return { espresso: 18, filter: 20 }[name] ?? 0;
  }

  describe(name) {
    return `${name}: ${Recipe.dose(name)}g beans`;
  }

  static preview(name) {
    return Report.render([name]);
  }
}
