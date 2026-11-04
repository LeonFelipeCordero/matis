const doses: Record<string, number> = { espresso: 16, filter: 22 };
let beans = 500;

export function reserve(name: string): string {
  const dose = doses[name] ?? 0;
  if (!dose || beans < dose) return `Unavailable decaf: ${name}`;
  beans -= dose;
  return `Reserved ${dose}g decaf for ${name}`;
}
