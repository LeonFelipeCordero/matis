import { Report } from "../../../reporting/Report.js";

function seasonalReceipt(names) {
  return Report.render(names.filter(name => name !== "unknown"));
}

export { seasonalReceipt as receipt };
export default seasonalReceipt;
