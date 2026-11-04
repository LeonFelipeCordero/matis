import type { Order } from "./Order";
import { priceOrder } from "./Orders";

export function trackingTotal(order: Order, /* delivery fee */ fee?: number): number {
  return priceOrder(order, 0, fee ?? 0);
}

export function trackingQuote({ items, ...details }: Order, fees = [0, 0], ...surcharges: number[]): number {
  return priceOrder({ ...details, items }, 0, ...fees, ...surcharges);
}
