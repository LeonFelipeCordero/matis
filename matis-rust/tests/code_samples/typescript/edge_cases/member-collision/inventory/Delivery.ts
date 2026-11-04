import type { Order } from "../../../ordering/Order.ts";

export function deliveryLabel(order: Order): string {
  const units = order.items.reduce((sum, { quantity = 1 }) => sum + quantity, 0);
  return `Deliver ${units} drinks to ${order.customer}`;
}

export function manifest(orders: Order[]): string[] {
  return orders.map(order => deliveryLabel(order));
}
