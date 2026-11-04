import { Recipe } from "../brewing/Recipe.ts";
import { priceOrder } from "../ordering/Orders.ts";
import type { Order, OrderItem } from "../ordering/Order.ts";

interface ReceiptFormatters {
  header: (order: Order, shop?: string) => string;
  money: (amount: number, currency?: string) => string;
  items: (order: Order) => Generator<string>;
  totals: (order: Order, discount?: number, ...fees: number[]) => Generator<string>;
}

export const receiptFormatters = {} as ReceiptFormatters;

receiptFormatters.header = (({ customer }: Order, shop = "Matis Coffee"): string => `${shop} / ${customer}`);
receiptFormatters.money = (function formatMoney(amount: number, currency = "EUR"): string {
  return `${currency} ${amount.toFixed(2)}`;
});
receiptFormatters.items = (function* itemLines({ items }: Order): Generator<string> {
  function describe({ name, quantity = 1 }: OrderItem): string {
    return `${quantity} x ${name}: ${receiptFormatters.money(Recipe.price(name) * quantity)}`;
  }
  for (const item of items) yield describe(item);
});
receiptFormatters.totals = (function* (order: Order, discount = 0, ...fees: number[]): Generator<string> {
  yield `Total: ${receiptFormatters.money(priceOrder(order, discount, ...fees))}`;
  if (order.note) yield `Note: ${order.note}`;
});

export class Receipt {
  constructor(private readonly shop = "Matis Coffee") {}

  render(order: Order, discount = 0, ...fees: number[]): string {
    return [
      receiptFormatters.header(order, this.shop),
      ...receiptFormatters.items(order),
      ...receiptFormatters.totals(order, discount, ...fees),
    ].join("\n");
  }
}
