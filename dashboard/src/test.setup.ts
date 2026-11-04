import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { buildRustCli } from '../tests/rust-cli'

// Compile once before tests start, outside their per-test time limits.
buildRustCli()

const directory = mkdtempSync(join(tmpdir(), 'matis-test-'))

process.env.DATABASE_PATH = join(directory, 'db')
process.on('exit', () => rmSync(directory, { recursive: true, force: true }))
