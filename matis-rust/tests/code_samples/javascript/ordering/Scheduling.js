import { Recipe } from "../brewing/Recipe.js";

export const scheduleDrink = ((name) => Recipe.portions(name));
export default scheduleDrink;
export const scheduleBatch = (function (names) {
  return names.map(scheduleDrink);
});
export const pickupLabel = (function label(customer) {
  return `pickup for ${customer}`;
});
export const scheduledTickets = (function* (names) {
  yield* names.map(scheduleDrink);
});
export const namedTickets = (function* tickets(names) {
  yield* scheduledTickets(names);
});

export function* schedulingQueue(orders) {
  const total = ((names) => names.reduce((sum, name) => sum + scheduleDrink(name), 0));
  for (const order of orders) {
    yield { customer: order.customer, portions: total(order.names) };
  }
}
