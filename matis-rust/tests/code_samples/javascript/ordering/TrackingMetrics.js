import { priceOrder } from "./Orders.js";

export function trackingQuote(order, [pickup, delivery] = [0, 0], discount = 0, ...fees) {
  return priceOrder(order, discount, pickup, delivery, ...fees);
}
export const trackingLabel = order => `${order.customer}: ${order.status}`;
export const trackingQueue = () => [];

export class PickupQueue {
  constructor() { this.orders = trackingQueue(); }
  record(order, status) {
    order.status = status;
    this.orders.push(order);
    return trackingLabel(order);
  }
  collect() {
    const ready = this.orders.filter(order => order.status === "ready");
    this.orders = this.orders.filter(order => order.status !== "ready");
    return ready;
  }
}
