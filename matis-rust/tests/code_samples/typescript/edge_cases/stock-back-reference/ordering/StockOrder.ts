import { stock } from "../inventory/Stock.ts";
export type StockOrder = { size: number };
export const type = stock;
export function orderStock() { return stock; }
export default orderStock;
