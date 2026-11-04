import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../../', import.meta.url))
const fixture = join(root, 'dashboard/tests/rust-fixture')
const target = join(fixture, 'target')
let built = false

function run(command: Array<string>, databasePath?: string) {
  const result = Bun.spawnSync(command, {
    cwd: root,
    env: {
      ...process.env,
      ...(databasePath ? { DATABASE_PATH: databasePath } : {}),
    },
  })
  if (result.exitCode !== 0)
    throw new Error(`${command.join(' ')} failed: ${result.stderr.toString()}`)
  return result.stdout.toString()
}

export function buildRustCli() {
  if (!built) {
    run([
      'cargo',
      'build',
      '--locked',
      '--manifest-path',
      join(fixture, 'Cargo.toml'),
      '--target-dir',
      target,
      '--bins',
    ])
    built = true
  }
}

function binary(name: string) {
  buildRustCli()
  return join(target, 'debug', name)
}

export function rustCli(databasePath: string, ...args: Array<string>) {
  return run([binary('matis'), ...args], databasePath)
}

export function migrateDatabase(databasePath: string) {
  run([binary('matis-migrate')], databasePath)
}

export function rustFixture(databasePath: string, ...args: Array<string>) {
  return run([binary('dashboard-matis-fixture'), ...args], databasePath)
}
