// Bound parameters keep each batch below SQLite's variable limit.
export function selectedRows<T>(
  keys: Array<string>,
  read: (batch: Array<string>) => Array<T>,
): Array<T> {
  const rows: Array<T> = []
  for (let start = 0; start < keys.length; start += 500)
    rows.push(...read(keys.slice(start, start + 500)))
  return rows
}
