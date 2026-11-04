// Order intake and recovery keep each receiver's lifetime local.
import {ReservationLedger, ReservationLedger as PickupLedger, DeliveryLedger,
        PickupStation, CounterStation, PluginLedger, PluginLedger as VendorLedger} from './Ledger';
import * as journals from './Ledger';

export function acceptUnknown(order, unknown) {
  return unknown.commit(order);
}

export function redirectInferred(order, replacement) {
  let ledger = new ReservationLedger();
  ledger = replacement;
  return ledger.commit(order);
}

export function replaceField(order, replacement) {
  const station = new PickupStation();
  station.ledger = replacement;
  return station.ledger.commit(order);
}

export function replaceAlias(order, replacement) {
  const station = new PickupStation();
  const alias = station;
  alias.ledger = replacement;
  return station.ledger.commit(order);
}

export function fulfillUnchanged(order) {
  const station = new CounterStation();
  return station.ledger.commit(order);
}

export function advanceReceiver(order) {
  let ledger = new ReservationLedger();
  ledger++;
  return ledger.commit(order);
}

export function restoreBatch(order, replacements) {
  let ledger = new ReservationLedger();
  [ledger] = replacements;
  return ledger.commit(order);
}

export function visitBatch(order, batches) {
  const station = new ReservationLedger();
  function visit(station) {
    return station.commit(order);
  }
  const receipts = batches.map(visit);
  station.commit(order);
  return receipts;
}

export function installVendor(order, replacement) {
  VendorLedger.prototype.commit = replacement;
  const ledger = new VendorLedger();
  return ledger.commit(order);
}

export function fulfill(order, ledger: journals.ReservationLedger) {
  return ledger.commit(order);
}

export function redirectDeclared(order, ledger: ReservationLedger) {
  ledger = new DeliveryLedger();
  return ledger.commit(order);
}

export function receiveUnion(order, ledger: ReservationLedger | DeliveryLedger) {
  return ledger.commit(order);
}

export function migrateVendor(order, ledger: PluginLedger, replacement) {
  VendorLedger.prototype.commit = replacement;
  return ledger.commit(order);
}

export function fulfillDeclared(order, ledger: ReservationLedger) {
  return ledger.commit(order);
}

export function openPickup(order) {
  const ledger: ReservationLedger = new ReservationLedger();
  return ledger.commit(order);
}

export function openDelivery(order) {
  const ledger: ReservationLedger = new DeliveryLedger();
  return ledger.commit(order);
}

export function openAlias(order) {
  const ledger: ReservationLedger = new PickupLedger();
  return ledger.commit(order);
}

export function resumePickup(order) {
  const opened = new ReservationLedger();
  const ledger: ReservationLedger = opened;
  return ledger.commit(order);
}

export function resumeDelivery(order) {
  const opened = new DeliveryLedger();
  const ledger: ReservationLedger = opened;
  return ledger.commit(order);
}
