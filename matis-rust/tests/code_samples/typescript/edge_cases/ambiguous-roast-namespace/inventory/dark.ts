export class Ambiguous {
  static reserve(name: string): string {
    const dose = name === "espresso" ? 16 : 22;
    return `Dark roast sample: ${name}, ${dose}g`;
  }
