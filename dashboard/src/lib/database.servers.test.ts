import { homedir } from 'node:os'
import { join } from 'node:path'
import { expect, test } from 'bun:test'
import { defaultDatabasePath } from './repositories.servers'

test('shared default database lives in user data with explicit overrides', () => {
  const databasePath = process.env.DATABASE_PATH
  const dataHome = process.env.XDG_DATA_HOME
  try {
    delete process.env.DATABASE_PATH
    delete process.env.XDG_DATA_HOME
    expect(defaultDatabasePath()).toBe(
      join(homedir(), '.local/share/matis/matis.sqlite'),
    )
    process.env.XDG_DATA_HOME = '/custom/user-data'
    expect(defaultDatabasePath()).toBe('/custom/user-data/matis/matis.sqlite')
    process.env.DATABASE_PATH = '/custom/database.sqlite'
    expect(defaultDatabasePath()).toBe('/custom/database.sqlite')
  } finally {
    if (databasePath === undefined) delete process.env.DATABASE_PATH
    else process.env.DATABASE_PATH = databasePath
    if (dataHome === undefined) delete process.env.XDG_DATA_HOME
    else process.env.XDG_DATA_HOME = dataHome
  }
})
