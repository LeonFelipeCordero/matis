import { Inventory } from "../../../inventory/Inventory.ts";
import type { Order } from "../../../ordering/Order.ts";

export function reserve(order: Order): string {
  const drinks = order.items.map(({ name, quantity = 1 }) => `${quantity} ${name}`).join(", ");
  return `Reserved for ${order.customer}: ${drinks}`;
}

export function queueOrder(inventory: Inventory, order: Order): string {
  if (!inventory.reserve(order)) return `Waiting for stock: ${order.customer}`;
  return reserve(order);
}

export function requestSupplier(supplier: any, order: Order): string {
  if (!supplier.reserve(order)) return `Supplier cannot fill: ${order.customer}`;
  return supplier.deliveryLabel(order);
}
