// Vendor migration entry point; handler writes invalidate this order's journal.
import {PluginLedger, PluginLedger as VendorLedger} from '../Ledger';

export function retainPrototype(order, replacement) {
  const ledger = new PluginLedger();
  VendorLedger.prototype.commit &&= replacement;
  return ledger.commit(order);
}
