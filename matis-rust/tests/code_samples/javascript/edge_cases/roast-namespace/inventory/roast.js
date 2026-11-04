const doses = { espresso: 18, filter: 24 };
let beans = 1000;

export class TastingFlight {
  names = [];

  reserve(name) {
    const dose = doses[name] ?? 0;
    if (!dose || beans < dose) return this;
    beans -= dose;
    this.names.push(name);
    return this;
  }
}

export function reserve(name) {
  return new TastingFlight().reserve(name);
}
