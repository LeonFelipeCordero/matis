import { Recipe } from "../brewing/Recipe.js";

export class Inventory {
  constructor(beans = 1000, cups = 50) {
    this.beans = beans;
    this.cups = cups;
  }

  canFill({ items }) {
    const beans = items.reduce((sum, { name, quantity = 1 }) => sum + Recipe.dose(name) * quantity, 0);
    const cups = items.reduce((sum, { quantity = 1 }) => sum + quantity, 0);
    return beans <= this.beans && cups <= this.cups;
  }

  reserve(order) {
    if (!this.canFill(order)) return false;
    for (const { name, quantity = 1 } of order.items) {
      this.beans -= Recipe.dose(name) * quantity;
      this.cups -= quantity;
    }
    return true;
  }

  restock({ beans = 0, cups = 0 } = {}) {
    this.beans += beans;
    this.cups += cups;
  }
}

export const stockReview = {
  remaining: ((inventory, reserve = 100) => Math.max(0, inventory.beans - reserve)),
  needsDelivery: (function belowMinimum(inventory, minimum = 200) {
    return inventory.beans < minimum;
  }),
  deliveries: (function* deliveryPlan(inventory, target = 1000) {
    function shortage(current) {
      return Math.max(0, target - current);
    }
    if (shortage(inventory.beans)) yield { beans: shortage(inventory.beans) };
  }),
  alerts: (function* (inventory) {
    if (inventory.beans < 200) yield "Order beans";
    if (inventory.cups < 10) yield "Order cups";
  }),
};

export function replenish(inventory = new Inventory()) {
  for (const delivery of stockReview.deliveries(inventory)) inventory.restock(delivery);
  return [...stockReview.alerts(inventory)];
}
