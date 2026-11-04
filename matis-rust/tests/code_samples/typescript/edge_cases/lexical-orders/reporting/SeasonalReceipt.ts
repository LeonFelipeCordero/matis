import { Report } from "../../../reporting/Report.ts";

function seasonalReceipt(names: string[]): string {
  return Report.render(names.filter(name => name !== "unknown"));
}

export { seasonalReceipt as receipt };
export default seasonalReceipt;
