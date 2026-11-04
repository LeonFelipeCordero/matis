// Vendor migration entry point; handler writes invalidate this order's journal.
import {PluginLedger, PluginLedger as VendorLedger} from '../Ledger';

export function fallbackInstance(order, replacement) {
  const ledger = new PluginLedger();
  ledger.commit ||= replacement;
  return ledger.commit(order);
}
