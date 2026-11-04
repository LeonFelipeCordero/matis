import { Recipe } from "../brewing/Recipe.ts";
import type { Order } from "../ordering/Order.ts";

export interface Delivery {
  beans?: number;
  cups?: number;
}

export class Inventory {
  constructor(public beans = 1000, public cups = 50) {}

  canFill({ items }: Order): boolean {
    const beans = items.reduce((sum, { name, quantity = 1 }) => sum + Recipe.dose(name) * quantity, 0);
    const cups = items.reduce((sum, { quantity = 1 }) => sum + quantity, 0);
    return beans <= this.beans && cups <= this.cups;
  }

  reserve(order: Order): boolean {
    if (!this.canFill(order)) return false;
    for (const { name, quantity = 1 } of order.items) {
      this.beans -= Recipe.dose(name) * quantity;
      this.cups -= quantity;
    }
    return true;
  }

  restock({ beans = 0, cups = 0 }: Delivery = {}): void {
    this.beans += beans;
    this.cups += cups;
  }
}

export const stockReview = {
  remaining: ((inventory: Inventory, reserve = 100): number => Math.max(0, inventory.beans - reserve)),
  needsDelivery: (function belowMinimum(inventory: Inventory, minimum = 200): boolean {
    return inventory.beans < minimum;
  }),
  deliveries: (function* deliveryPlan(inventory: Inventory, target = 1000): Generator<Delivery> {
    function shortage(current: number): number {
      return Math.max(0, target - current);
    }
    if (shortage(inventory.beans)) yield { beans: shortage(inventory.beans) };
  }),
  alerts: (function* (inventory: Inventory): Generator<string> {
    if (inventory.beans < 200) yield "Order beans";
    if (inventory.cups < 10) yield "Order cups";
  }),
};

export function replenish(inventory: Inventory = new Inventory()): string[] {
  for (const delivery of stockReview.deliveries(inventory)) inventory.restock(delivery);
  return [...stockReview.alerts(inventory)];
}
