import { Brewer } from "../brewing/Brewer.ts";
import { Recipe } from "../brewing/Recipe.ts";
import { Inventory as BeanInventory } from "../inventory/Inventory.ts";
import { Receipt } from "../reporting/Receipt.ts";
import type { Order as CoffeeOrder } from "./Order.ts";
import { drinkTickets, createOrder } from "./Orders.ts";

export class OrderService {
  private readonly recipe: Recipe = new Recipe();

  constructor(
    private readonly inventory: BeanInventory = new BeanInventory(),
    private readonly brewer: Brewer = new Brewer(),
    private readonly receipt: Receipt = new Receipt(),
  ) {}

  fulfill(order: CoffeeOrder) {
    if (!this.inventory.reserve(order)) return { status: "out-of-stock" as const };
    const names = [...drinkTickets(order)].map(ticket => ticket.name);
    const batch = this.brewer.brew(names);
    order.status = "ready";
    return { status: order.status, batch, receipt: this.receipt.render(order) };
  }

  describe(order: CoffeeOrder): string {
    return order.items.map(({ name }) => this.recipe.describe(name)).join("\n");
  }
}

export function morningOrder(customer = "Ada") {
  const inventory: BeanInventory = new BeanInventory();
  inventory.restock({ beans: 250 });
  const counter: OrderService = new OrderService(inventory);
  return counter.fulfill(createOrder("morning-1", customer, [{ name: "espresso", quantity: 2 }]));
}

export function checkStock(inventory: BeanInventory, order: CoffeeOrder): boolean {
  return inventory.canFill(order);
}

export { OrderService as CoffeeCounter };
