import * as seasonal from "../reporting/SeasonalReceipt.js";
import { receipt as printReceipt } from "../reporting/SeasonalReceipt.js";

export function receipt(names) {
  const named = printReceipt(names);
  const defaultReceipt = seasonal.default(names);
  const exportedReceipt = seasonal.receipt(names);
  const privateReceipt = seasonal.seasonalReceipt(names);
  return { named, defaultReceipt, exportedReceipt, privateReceipt };
}

export function unlinked(names) {
  return new Brewer().brew(names);
}
