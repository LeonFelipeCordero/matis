export interface OrderItem {
  name: string;
  quantity?: number;
}

export interface Order {
  id: string;
  customer: string;
  items: OrderItem[];
  note?: string;
  status: "queued" | "ready";
}

export interface Ticket {
  name: string;
  index: number;
  dose: number;
}

export interface OrderBook {
  select: <T>(project: (order: Order) => T) => T[];
  onChange?: ((((order: Order, index: number) => void)));
  "daily-total": (discount?: number) => number;
  [Symbol.iterator]: () => Iterator<Order>;
  find(id: string): Order | undefined;
  find(predicate: (order: Order) => boolean): Order | undefined;
  count: number;
}

export interface PricingContext {
  serviceFee: number;
}

export interface OrderBookProvider {
  notifications: { callback: (book: OrderBook) => void };
  factory: new () => OrderBook;
}
