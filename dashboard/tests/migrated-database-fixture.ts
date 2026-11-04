import { copyFileSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { afterAll, beforeAll } from 'bun:test'
import { Database } from 'bun:sqlite'

import { migrateDatabase } from './rust-cli'

// Copy a closed Rust-migrated template, never a database shared by tests.
export function migratedDatabaseFixture() {
  let directory: string
  let template: string
  beforeAll(() => {
    directory = mkdtempSync(join(tmpdir(), 'matis-migrated-template-'))
    template = join(directory, 'template.sqlite')
    migrateDatabase(template)
    const database = new Database(template, { readwrite: true, create: false })
    try {
      database.exec('PRAGMA wal_checkpoint(TRUNCATE)')
    } finally {
      database.close()
    }
  })
  afterAll(() => {
    if (directory) rmSync(directory, { recursive: true, force: true })
  })
  return (path: string) => {
    mkdirSync(dirname(path), { recursive: true })
    copyFileSync(template, path)
  }
}
