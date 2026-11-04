import { drinkQueue, readyOrders, trackingActions } from "./TrackingCarriers.js";

export function trackingView(orders) {
  return [...readyOrders(orders)].map(order => ({
    customer: order.customer,
    drinks: [...drinkQueue(order)],
    receipt: trackingActions.receipt(order),
  }));
}
