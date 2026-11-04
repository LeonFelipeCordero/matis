import {ReservationLedger} from './Ledger';

export class PickupPanel {
  ledger = new ReservationLedger();
  fulfill(order) {
    return this.ledger.commit(order);
  }
}
