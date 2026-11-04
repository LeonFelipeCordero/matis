import type { Order, OrderBook } from "./Order.ts";
import { priceOrder } from "./Orders.ts";

export class OrderLedger implements OrderBook {
  onChange?: (order: Order, index: number) => void;

  constructor(private readonly orders: Order[] = []) {}

  get count(): number {
    return this.orders.length;
  }

  select<T>(project: (order: Order) => T): T[] {
    return this.orders.map(project);
  }

  "daily-total"(discount = 0): number {
    return this.orders.reduce((sum, order) => sum + priceOrder(order, discount), 0);
  }

  *[Symbol.iterator](): Generator<Order> {
    yield* this.orders;
  }

  find(id: string): Order | undefined;
  find(predicate: (order: Order) => boolean): Order | undefined;
  find(query: string | ((order: Order) => boolean)): Order | undefined {
    return this.orders.find(typeof query === "string" ? order => order.id === query : query);
  }

  add(order: Order): void {
    this.orders.push(order);
    this.onChange?.(order, this.count - 1);
  }
}

export { OrderLedger as DailyOrders };
