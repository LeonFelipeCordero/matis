import { drinkQueue, readyOrders, trackingActions, type Order } from "./TrackingCarriers";

export function TrackingView({ orders }: { orders: Order[] }) {
  return <section aria-label="Ready for pickup">
    {[...readyOrders(orders)].map(order => <article key={order.customer}>
      <h2>{order.customer}</h2>
      <p>{[...drinkQueue(order)].join(", ")}</p>
      <small>{trackingActions.receipt(order)}</small>
    </article>)}
  </section>;
}
