import type { BasketItem } from "./Basket";

export interface Order {
  customer: string;
  items: BasketItem[];
  status: "queued" | "ready";
  note?: string;
}

// Dispatch café orders from the brew queue through pickup and receipt delivery.
export const prepareOrder = (((order: Order) => {
  function hasDrinks() { return order.items.length > 0; }
  if (hasDrinks()) order.status = "ready";
  return order;
}));

export const pickupLabel = ((function labelPickup(order: Order) {
  function customerLabel() { return `${order.customer}: ${order.note || "counter"}`; }
  return customerLabel();
}));

export const drinkQueue = (function* expandDrinks(order: Order) {
  function drinkLabel(name: string, index: number) { return `${name} #${index + 1}`; }
  for (const item of order.items) {
    for (let index = 0; index < item.quantity; index++) yield drinkLabel(item.name, index);
  }
});

export const readyOrders = (((function* (orders: Order[]) {
  function ready(order: Order) { return order.status === "ready"; }
  for (const order of orders) if (ready(order)) yield order;
})));

export const tracking = {} as {
  hold: (order: Order, reason: string) => string;
  requeue: (order: Order) => Order;
  audit: (orders: Order[]) => Generator<Order>;
  archive: (orders: Order[]) => Generator<string>;
};

tracking.hold = ((order: Order, reason: string) => {
  function holdNote() { return `Hold ${order.customer}: ${reason}`; }
  order.note = holdNote();
  return order.note;
});

tracking.requeue = (((function retryPickup(order: Order) {
  function resetPickup() { order.status = "queued"; order.note = "retry at counter"; return order; }
  return resetPickup();
})));

tracking.audit = ((function* missingDrinks(orders: Order[]) {
  function empty(order: Order) { return order.items.length === 0; }
  for (const order of orders) if (empty(order)) yield order;
}));

tracking.archive = (function* (orders: Order[]) {
  function archiveEntry(order: Order) { return `${order.customer} (${order.status})`; }
  for (const order of orders) yield archiveEntry(order);
});

export const trackingActions = {
  collect: (((orders: Order[], order: Order) => {
    function queuePosition() { return orders.indexOf(order); }
    const index = queuePosition();
    if (index >= 0) orders.splice(index, 1);
    return order;
  })),
  receipt: (function printReceipt(order: Order) {
    function itemCount() { return order.items.reduce((sum, item) => sum + item.quantity, 0); }
    return `${order.customer}: ${itemCount()} drinks`;
  }),
  notifications: ((function* pickupNotifications(orders: Order[]) {
    function notification(order: Order) { return `Pickup for ${order.customer} at ${order.note || "counter"}`; }
    for (const order of orders) yield notification(order);
  })),
  deliveries: (((function* (orders: Order[]) {
    function delivery(order: Order) { return { customer: order.customer, drinks: order.items, note: order.note }; }
    for (const order of orders) if (order.status === "ready") yield delivery(order);
  }))),
};

export function* trackBatch(orders: Order[]) {
  const collectTicket = (((order: Order) => {
    function batchLabel() { return `${order.customer}: ${order.items.length} lines`; }
    return batchLabel();
  }));
  for (const order of orders) yield collectTicket(order);
}

export function consumeTicket(order: Order, callback: (order: Order) => string) { return callback(order); }

export function dispatchOrders(orders: Order[]) {
  for (const order of tracking.audit(orders)) tracking.hold(order, "missing drinks");
  for (const order of orders) if (order.note === "retry") tracking.requeue(order);
  for (const order of orders) prepareOrder(order);
  const ready = [...readyOrders(orders)];
  const labels = ready.map(order => consumeTicket(order, ((function internalTicket(order: Order) {
    function dispatchLabel() { return `${order.customer}: ready for collection`; }
    return dispatchLabel();
  }))));
  const summary = ready.map(order => consumeTicket(order, ((order: Order) => {
    function queueLabel() { return `${order.customer}: ${order.status}`; }
    return queueLabel();
  })));
  const result = {
    labels, summary, pickups: ready.map(pickupLabel), drinks: ready.map(order => [...drinkQueue(order)]),
    batches: [...trackBatch(ready)], notifications: [...trackingActions.notifications(ready)],
    deliveries: [...trackingActions.deliveries(ready)], receipts: ready.map(trackingActions.receipt),
    archive: [...tracking.archive(ready)],
  };
  for (const order of ready) trackingActions.collect(orders, order);
  return result;
}
