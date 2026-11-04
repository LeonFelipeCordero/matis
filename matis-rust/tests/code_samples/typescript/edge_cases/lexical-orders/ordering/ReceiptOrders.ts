import * as seasonal from "../reporting/SeasonalReceipt.ts";
import { receipt as printReceipt } from "../reporting/SeasonalReceipt.ts";
import { Report } from "../../../reporting/Report.ts";

export function receipt(names: string[]) {
  const named = printReceipt(names);
  const defaultReceipt = seasonal.default(names);
  const exportedReceipt = seasonal.receipt(names);
  const privateReceipt = seasonal.seasonalReceipt(names);
  return { named, defaultReceipt, exportedReceipt, privateReceipt };
}

export function unlinked(names: string[]) {
  return new Brewer().brew(names);
}

class ReceiptPrinter {
  render(names: string[]) { return Report.render(names); }
}

export function replacedPrototype(names: string[], replacement) {
  ReceiptPrinter.prototype = replacement;
  return new ReceiptPrinter().render(names);
}

export function replacedMethod(names: string[], replacement) {
  ReceiptPrinter.prototype.render = replacement;
  return new ReceiptPrinter().render(names);
}
