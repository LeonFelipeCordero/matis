// Dispatch café orders from the brew queue through pickup and receipt delivery.
export const prepareOrder = (((order) => {
  function hasDrinks() { return order.items.length > 0; }
  if (hasDrinks()) order.status = "ready";
  return order;
}));

export const pickupLabel = ((function labelPickup(order) {
  function customerLabel() { return `${order.customer}: ${order.note || "counter"}`; }
  return customerLabel();
}));

export const drinkQueue = (function* expandDrinks(order) {
  function drinkLabel(name, index) { return `${name} #${index + 1}`; }
  for (const item of order.items) {
    for (let index = 0; index < (item.quantity ?? 1); index++) yield drinkLabel(item.name, index);
  }
});

export const readyOrders = (((function* (orders) {
  function ready(order) { return order.status === "ready"; }
  for (const order of orders) if (ready(order)) yield order;
})));

export const tracking = {};

tracking.hold = ((order, reason) => {
  function holdNote() { return `Hold ${order.customer}: ${reason}`; }
  order.note = holdNote();
  return order.note;
});

tracking.requeue = (((function retryPickup(order) {
  function resetPickup() { order.status = "queued"; order.note = "retry at counter"; return order; }
  return resetPickup();
})));

tracking.audit = ((function* missingDrinks(orders) {
  function empty(order) { return order.items.length === 0; }
  for (const order of orders) if (empty(order)) yield order;
}));

tracking.archive = (function* (orders) {
  function archiveEntry(order) { return `${order.customer} (${order.status})`; }
  for (const order of orders) yield archiveEntry(order);
});

export const trackingActions = {
  collect: (((orders, order) => {
    function queuePosition() { return orders.indexOf(order); }
    const index = queuePosition();
    if (index >= 0) orders.splice(index, 1);
    return order;
  })),
  receipt: (function printReceipt(order) {
    function itemCount() { return order.items.reduce((sum, item) => sum + (item.quantity ?? 1), 0); }
    return `${order.customer}: ${itemCount()} drinks`;
  }),
  notifications: ((function* pickupNotifications(orders) {
    function notification(order) { return `Pickup for ${order.customer} at ${order.note || "counter"}`; }
    for (const order of orders) yield notification(order);
  })),
  deliveries: (((function* (orders) {
    function delivery(order) { return { customer: order.customer, drinks: order.items, note: order.note }; }
    for (const order of orders) if (order.status === "ready") yield delivery(order);
  }))),
};

export function* trackBatch(orders) {
  const collectTicket = (((order) => {
    function batchLabel() { return `${order.customer}: ${order.items.length} lines`; }
    return batchLabel();
  }));
  for (const order of orders) yield collectTicket(order);
}

export function consumeTicket(order, callback) { return callback(order); }

export function dispatchOrders(orders) {
  for (const order of tracking.audit(orders)) tracking.hold(order, "missing drinks");
  for (const order of orders) if (order.note === "retry") tracking.requeue(order);
  for (const order of orders) prepareOrder(order);
  const ready = [...readyOrders(orders)];
  const labels = ready.map(order => consumeTicket(order, ((function internalTicket(order) {
    function dispatchLabel() { return `${order.customer}: ready for collection`; }
    return dispatchLabel();
  }))));
  const summary = ready.map(order => consumeTicket(order, ((order) => {
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
