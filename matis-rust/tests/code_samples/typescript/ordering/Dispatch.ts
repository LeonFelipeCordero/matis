import { Receipt } from "../reporting/Receipt.ts";
import type { Order } from "./Order.ts";
import type * as orderTypes from "./Order.ts";

export interface DispatchBook extends orderTypes.OrderBook, Iterable<Order> {
  pending: number;
  deliver: (send: (receipt: string) => void) => number;
}

export class Dispatch extends Receipt implements Iterable<Order> {
  private orders: Order[] = [];

  enqueue(order: Order): void {
    this.orders.push(order);
  }

  *[Symbol.iterator](): Iterator<Order> {
    yield* this.orders;
  }

  deliver(send: (receipt: string) => void): number {
    const ready = this.orders.filter(order => order.status === "ready");
    ready.forEach(order => send(this.render(order)));
    return ready.length;
  }
}
