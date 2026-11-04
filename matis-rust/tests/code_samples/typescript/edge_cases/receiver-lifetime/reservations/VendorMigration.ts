// Legacy vendor handler migration; each entry point has its own receiver.
import {PluginLedger, PluginLedger as VendorLedger} from './Ledger';

export function replaceMember(order, replacement) {
  const ledger = new PluginLedger();
  ledger.commit = replacement;
  return ledger.commit(order);
}
