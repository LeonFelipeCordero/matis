import { Recipe } from "../brewing/Recipe.js";

export const priceOrder = (({ items }, discount = 0, ...fees) => {
  function lineTotal({ name, quantity = 1 }) {
    return Recipe.price(name) * quantity;
  }
  return items.reduce((sum, item) => sum + lineTotal(item), 0) * (1 - discount)
    + fees.reduce((sum, fee) => sum + fee, 0);
});

export const createOrder = (function buildOrder(customer, [first, ...others], note = "") {
  return { customer, items: [first, ...others], note, status: "queued" };
});

export const drinkTickets = (function* expandTickets({ items }) {
  function ticket(name, index) {
    return { name, index, dose: Recipe.dose(name) };
  }
  for (const { name, quantity = 1 } of items) {
    for (let index = 0; index < quantity; index++) yield ticket(name, index);
  }
});

export const pickupQueue = (function* (orders, status = "ready") {
  for (const order of orders) {
    if (order.status === status) yield order;
  }
});

export function groupTickets(orders) {
  return orders.map(order => [...drinkTickets(order)].map(ticket => ({
    ...ticket, customer: order.customer,
  })));
}

export { createOrder as placeOrder };
