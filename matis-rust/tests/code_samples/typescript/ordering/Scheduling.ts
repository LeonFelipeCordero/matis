import { Recipe } from "../brewing/Recipe.ts";

export const scheduleDrink = ((name: string): number => Recipe.portions(name));
export const scheduleBatch = (function (names: string[]): number[] {
  return names.map(scheduleDrink);
});
export const pickupLabel = (function label(customer: string): string {
  return `pickup for ${customer}`;
});
export const scheduledTickets = (function* (names: string[]) {
  yield* names.map(scheduleDrink);
});
export const namedTickets = (function* tickets(names: string[]) {
  yield* scheduledTickets(names);
});

export function* schedulingQueue(orders: { customer: string; names: string[] }[]) {
  const total = ((names: string[]) => names.reduce((sum, name) => sum + scheduleDrink(name), 0));
  for (const order of orders) {
    yield { customer: order.customer, portions: total(order.names) };
  }
}
