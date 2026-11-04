import { Brewer } from "../brewing/Brewer.js";
import { Recipe } from "../brewing/Recipe.js";
import { Inventory } from "../inventory/Inventory.js";
import { Receipt } from "../reporting/Receipt.js";
import { drinkTickets, createOrder } from "./Orders.js";

export class OrderService {
  constructor(inventory = new Inventory(), brewer = new Brewer(), receipt = new Receipt()) {
    this.inventory = inventory;
    this.brewer = brewer;
    this.receipt = receipt;
    this.recipe = new Recipe();
  }

  fulfill(order) {
    if (!this.inventory.reserve(order)) return { status: "out-of-stock" };
    const names = [...drinkTickets(order)].map(ticket => ticket.name);
    const batch = this.brewer.brew(names);
    order.status = "ready";
    return { status: order.status, batch, receipt: this.receipt.render(order) };
  }

  describe(order) {
    return order.items.map(({ name }) => this.recipe.describe(name)).join("\n");
  }
}

export function morningOrder(customer = "Ada") {
  const inventory = new Inventory();
  inventory.restock({ beans: 250 });
  const counter = new OrderService(inventory);
  return counter.fulfill(createOrder(customer, [{ name: "espresso", quantity: 2 }]));
}

export { OrderService as CoffeeCounter };
