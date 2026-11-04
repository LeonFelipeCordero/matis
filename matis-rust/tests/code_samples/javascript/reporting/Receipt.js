import { Recipe } from "../brewing/Recipe.js";
import { priceOrder } from "../ordering/Orders.js";

export const receiptFormatters = {};

receiptFormatters.header = (({ customer }, shop = "Matis Coffee") => `${shop} / ${customer}`);
receiptFormatters.money = (function formatMoney(amount, currency = "EUR") {
  return `${currency} ${amount.toFixed(2)}`;
});
receiptFormatters.items = (function* itemLines({ items }) {
  function describe({ name, quantity = 1 }) {
    return `${quantity} x ${name}: ${receiptFormatters.money(Recipe.price(name) * quantity)}`;
  }
  for (const item of items) yield describe(item);
});
receiptFormatters.totals = (function* (order, discount = 0, ...fees) {
  yield `Total: ${receiptFormatters.money(priceOrder(order, discount, ...fees))}`;
  if (order.note) yield `Note: ${order.note}`;
});

export class Receipt {
  constructor(shop = "Matis Coffee") {
    this.shop = shop;
  }

  render(order, discount = 0, ...fees) {
    return [
      receiptFormatters.header(order, this.shop),
      ...receiptFormatters.items(order),
      ...receiptFormatters.totals(order, discount, ...fees),
    ].join("\n");
  }
}
