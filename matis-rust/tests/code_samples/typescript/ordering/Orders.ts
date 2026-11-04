import { Recipe } from "../brewing/Recipe.ts";
import type { Order, OrderItem, PricingContext, Ticket } from "./Order.ts";

export const priceOrder = (({ items }: Order, discount = 0, ...fees: number[]): number => {
  function lineTotal({ name, quantity = 1 }: OrderItem): number {
    return Recipe.price(name) * quantity;
  }
  return items.reduce((sum, item) => sum + lineTotal(item), 0) * (1 - discount)
    + fees.reduce((sum, fee) => sum + fee, 0);
});

export const createOrder = (function buildOrder(id: string, customer: string, [first, ...others]: [OrderItem, ...OrderItem[]], note = ""): Order {
  return { id, customer, items: [first, ...others], note, status: "queued" };
});

export const drinkTickets = (function* expandTickets({ items }: Order): Generator<Ticket> {
  function ticket(name: string, index: number): Ticket {
    return { name, index, dose: Recipe.dose(name) };
  }
  for (const { name, quantity = 1 } of items) {
    for (let index = 0; index < quantity; index++) yield ticket(name, index);
  }
});

export const pickupQueue = (function* (orders: Order[], status: Order["status"] = "ready"): Generator<Order> {
  for (const order of orders) {
    if (order.status === status) yield order;
  }
});

export function quote(order: Order): number;
export function quote(name: string, quantity: number): number;
export function quote(orderOrName: Order | string, quantity = 1): number {
  return typeof orderOrName === "string"
    ? Recipe.price(orderOrName) * quantity
    : priceOrder(orderOrName);
}

export function quoteWithFees<T extends Order>(this: PricingContext, { items, ...details }: T, discount?: number, ...fees: number[]): number {
  return priceOrder({ ...details, items }, discount, this.serviceFee, ...fees);
}

export function groupTickets(orders: Order[]) {
  return orders.map(order => [...drinkTickets(order)].map(ticket => ({
    ...ticket, customer: order.customer,
  })));
}

export { createOrder as placeOrder };
