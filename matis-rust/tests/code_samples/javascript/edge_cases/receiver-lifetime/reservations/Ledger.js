// Pickup and delivery journals share the order commit protocol.
export class ReservationLedger {
  orders = [];
  commit(order) {
    this.orders.push(order);
    return order;
  }
}

export class DeliveryLedger extends ReservationLedger {
  commit(order) {
    this.orders.push(order);
    return order.deliveryReceipt();
  }
}

export class PluginLedger {
  commit(order) {
    return order.enqueue("vendor");
  }
}

export class PickupStation {
  ledger = new ReservationLedger();
  fulfill(order) {
    return this.ledger.commit(order);
  }
  transfer(order, replacement) {
    const alias = this;
    alias.ledger = replacement;
    return this.ledger.commit(order);
  }
}

export class RecoveryStation {
  constructor() {
    this.ledger = new ReservationLedger();
  }
  redirect(order, replacement) {
    this.ledger = replacement;
    return this.ledger.commit(order);
  }
}

export class CounterStation {
  ledger = new ReservationLedger();
  fulfill(order) {
    return this.ledger.commit(order);
  }
}

export class OpeningStation {
  constructor() {
    this.ledger = new ReservationLedger();
  }
  fulfill(order) {
    return this.ledger.commit(order);
  }
}
