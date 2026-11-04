import type { Order } from "./Order";
import { drinkQueue, readyOrders, trackingActions } from "./TrackingCarriers";

export function trackingView(orders: Order[]) {
  return [...readyOrders(orders)].map(order => ({
    customer: order.customer,
    drinks: [...drinkQueue(order)],
    receipt: trackingActions.receipt(order),
  }));
}
