import { drinkTickets, priceOrder } from "./Orders.js";
import { Receipt } from "../reporting/Receipt.js";

export const dispatchLabel = order => "pickup:" + order.customer;
export const dispatchTickets = function* expandDispatch(order) {
  yield* drinkTickets(order);
};

export class Dispatch extends Receipt {
  constructor() {
    super();
    this.events = [];
    this.notify = function (order) { this.events.push(dispatchLabel(order)); };
    this.handlers = { archive: function* (orders) {
      for (const order of orders) yield dispatchLabel(order);
    } };
  }

  schedule(orders) {
    const collect = orders.map(order => {
      function ticketLabel(ticket) { return order.customer + ":" + ticket.name; }
      return [...dispatchTickets(order)].map(ticket => ticketLabel(ticket));
    });
    this.visit(orders, () => this.events.length, function () { return collect.length; },
      function* () { yield collect; }, function dispatchTotal() { return orders.reduce((sum, order) => sum + priceOrder(order), 0); });
    return function () { return collect; };
  }

  visit(orders, ...observers) {
    for (const order of orders) this.notify(order);
    for (const observe of observers) observe();
  }
}
