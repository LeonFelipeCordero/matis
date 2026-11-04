import { homedir } from 'node:os'
import { join, resolve } from 'node:path'
import { Database } from 'bun:sqlite'
import { drizzle } from 'drizzle-orm/bun-sqlite'

export type AppDatabase = Pick<
  ReturnType<typeof drizzle>,
  'select' | 'insert' | 'transaction'
>

export const defaultDatabasePath = () =>
  process.env.DATABASE_PATH ||
  join(
    process.env.XDG_DATA_HOME || join(homedir(), '.local/share'),
    'matis/matis.sqlite',
  )

/** Infrastructure only: Python creates/migrates the file; never create a UI schema. */
export function openConnection(path = defaultDatabasePath()) {
  const driver = new Database(resolve(path), {
    readwrite: true,
    create: false,
    strict: true,
  })
  driver.run('PRAGMA busy_timeout = 30000')
  driver.run('PRAGMA foreign_keys = ON')
  return { database: drizzle(driver), close: () => driver.close() }
}
