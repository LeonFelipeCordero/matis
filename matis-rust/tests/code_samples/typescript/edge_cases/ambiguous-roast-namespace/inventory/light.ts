export class Ambiguous {
  static reserve(name: string): string {
    const dose = name === "espresso" ? 18 : 24;
    return `Light roast sample: ${name}, ${dose}g`;
  }
}
