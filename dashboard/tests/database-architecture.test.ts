import { readFileSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join, relative } from 'node:path'
import { expect, test } from 'bun:test'
import ts from 'typescript-ast'
import {
  auditServiceExports,
  auditSource,
  repositoryRoot,
  serviceFile,
} from './database-architecture'

import { ownership } from './database-ownership-policy'

const root = fileURLToPath(new URL('..', import.meta.url))
function productionFiles(directory = root): Array<string> {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    if (entry.isDirectory())
      return ['node_modules', 'tests', 'public'].includes(entry.name) ||
        entry.name.startsWith('.')
        ? []
        : productionFiles(path)
    return /\.[cm]?[jt]sx?$/.test(entry.name) &&
      !/\.(?:test|spec)\./.test(entry.name)
      ? [path]
      : []
  })
}

test('all dashboard production follows service -> repository -> ORM, including bootstrap/config', () => {
  const files = productionFiles()
  expect(files.some((file) => file.endsWith('/src/server.ts'))).toBe(true)
  expect(files.some((file) => file.endsWith('/vite.config.ts'))).toBe(true)
  expect(
    files.flatMap((file) =>
      auditSource(relative(root, file), readFileSync(file, 'utf8')),
    ),
  ).toEqual([])
})

test('table repositories cannot depend on capability services', () => {
  for (const source of [
    "import { NodeService } from '../../services/node-service'",
    "export { NodeService } from '@/lib/services/node-service'",
    "const service = await import('@/lib/services/node-service')",
  ])
    expect(
      auditSource(`${repositoryRoot}node-repository.ts`, source).length,
    ).toBeGreaterThan(0)
})

test('service dependencies are acyclic', () => {
  const config = ts.readConfigFile(join(root, 'tsconfig.json'), ts.sys.readFile)
  const { options } = ts.parseJsonConfigFileContent(config.config, ts.sys, root)
  const files = productionFiles(join(root, 'src/lib/services'))
  const edges = new Map(
    files.map((file) => [
      file,
      ts
        .preProcessFile(readFileSync(file, 'utf8'), true)
        .importedFiles.map(
          ({ fileName }) =>
            ts.resolveModuleName(fileName, file, options, ts.sys).resolvedModule
              ?.resolvedFileName,
        )
        .filter((target): target is string => target !== undefined),
    ]),
  )
  const active = new Set<string>()
  const complete = new Set<string>()
  const visit = (file: string) => {
    if (complete.has(file)) return
    expect(active.has(file), `Cyclic service dependency at ${file}`).toBe(false)
    active.add(file)
    for (const target of edges.get(file) ?? [])
      if (edges.has(target)) visit(target)
    active.delete(file)
    complete.add(file)
  }
  files.forEach(visit)
})

test('each table and port permits only its exact owner, including aliases and relative paths', () => {
  for (const [table, repository, port, owner] of ownership) {
    const repositoryFile = `${repositoryRoot}${repository}.ts`
    const ownerFile = `src/lib/services/${owner}.ts`
    const wrongOwner =
      owner === 'node-service' ? 'node-metrics-service' : 'node-service'
    const tableImport = `import { ${table} as table } from './schema'`
    const portImport = `import type { ${port} as Port } from '../repository-ports'`
    expect(auditSource(repositoryFile, tableImport)).toEqual([])
    expect(auditSource(ownerFile, portImport)).toEqual([])
    expect(
      auditSource(`src/lib/services/${wrongOwner}.ts`, portImport).length,
    ).toBeGreaterThan(0)
    expect(
      auditSource(`${repositoryRoot}factory.ts`, tableImport).length,
    ).toBeGreaterThan(0)
    const bypasses = [
      `import { ${table} as table } from '@/lib/persistence/repositories/schema'`,
      `import { ${table} as table } from '../persistence/repositories/../repositories/schema.js'`,
      `import * as tables from '@/lib/persistence/repositories/schema'`,
      `const tables = await import('@/lib/persistence/repositories/schema')`,
      `type Tables = import('@/lib/persistence/repositories/schema')`,
      `export { ${table} as table } from '@/lib/persistence/repositories/schema'`,
    ]
    expect(
      bypasses.every((source) => auditSource(ownerFile, source).length > 0),
    ).toBe(true)
    expect(
      auditSource(`${repositoryRoot}another-repository.ts`, tableImport).length,
    ).toBeGreaterThan(0)
    expect(
      auditSource(
        repositoryFile,
        `import { Other } from './node-metrics-repository'`,
      ).length,
    ).toBeGreaterThan(0)
  }
  for (const file of ['factory.ts', 'schema.ts', 'alternate-repository.ts'])
    expect(
      auditSource(`${repositoryRoot}${file}`, 'database.select().from(table)')
        .length,
    ).toBeGreaterThan(0)
  expect(
    auditSource(
      `${repositoryRoot}node-repository.ts`,
      "import { sqliteTable } from 'drizzle-orm/sqlite-core'",
    ).length,
  ).toBeGreaterThan(0)
})

test('service exports only operations, DTOs and lifecycle, never persistence handles', () => {
  const config = ts.readConfigFile(join(root, 'tsconfig.json'), ts.sys.readFile)
  const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, root)
  const program = ts.createProgram([join(root, serviceFile)], parsed.options)
  expect(
    auditServiceExports(
      program,
      program.getSourceFile(join(root, serviceFile))!,
    ),
  ).toEqual([])
}, 30_000)

const failures = [
  "import { GraphStore as Renamed } from '@/lib/persistence/repositories/graph-store'; new Renamed(db)",
  "import type { AppDatabase as Handle } from '../lib/persistence/repositories/connection'",
  "export * from '@/lib/persistence/repositories/schema'",
  "import { createRepositoryContext as bypass } from '@/lib/persistence/repositories/factory'; bypass()",
  "export * from '@/lib/persistence/repositories'",
  "import { GraphStore } from '@/lib/persistence/repositories/../repositories/graph-store'",
  "import { createRepositoryContext } from '/absolute/src/lib/persistence/repositories/factory'",
  "export { RepositoryStore as Safe } from '../lib/persistence/repositories/repository-store'",
  "const load = () => import('@/lib/persistence/repositories/connection')",
  "type Handle = import('@/lib/persistence/repositories/connection').AppDatabase",
  "import Store = require('@/lib/persistence/repositories/repository-store')",
  "import { drizzle as mapped } from 'drizzle-orm/bun-sqlite'; mapped(db)",
  "import type { BunSQLiteDatabase as DTO } from 'drizzle-orm/bun-sqlite'",
  "export * from 'bun:sqlite'",
  "const driver = await import('node:sqlite')",
  "import fixture from './data.sqlite' with { type: 'sqlite' }",
  "import { SQL as Renamed } from 'bun'; new Renamed()",
  "new Bun.SQL('sqlite://test.db')",
  "new Bun['SQL']('sqlite://test.db')",
  'new Bun[driver]()',
  "new Bun['SQ' + 'L']()",
  'const runtime = Bun; const ctor = runtime[driver]; new ctor()',
  "process.getBuiltinModule('node:sqlite')",
  "const builtin = process.getBuiltinModule; builtin('node:sqlite')",
  'const method = db[computed]; method(query)',
  'const alias = db; const method = alias[computed]; method(query)',
  "Reflect.get(Bun, 'SQL')",
  'Reflect.construct(Database, [])',
  'Reflect.apply(db.query, db, [query])',
  "const reflection = Reflect; reflection.get(Bun, 'SQL')",
  "const { get: lookup } = Reflect; lookup(db, 'query')",
  "Object.getOwnPropertyDescriptor(db, 'query')",
  'const lookup = Object.getOwnPropertyDescriptors; lookup(db)',
  'const reflection = Object; reflection[computed](db)',
  'new Database()',
  'new DatabaseSync()',
  'const Driver = DatabaseSync; new Driver()',
  'db.update(table).set(values)',
  'db.delete(table)',
  "const alias = db; alias['delete'](table)",
  'const { update: change } = db; change(table)',
  "const { 'delete': remove } = db; remove(table)",
  'const { [computed]: method } = db; method(query)',
  'Object.getPrototypeOf(db)',
  "const arbitrary = require; arbitrary('bun:sqlite')",
  "import { createRequire as arbitrary } from 'node:module'; arbitrary(import.meta.url)('bun:sqlite')",
  'const load = import(location)',
  "db.query('SELECT id FROM repositories')",
  "db['prepare']('SELECT id FROM repositories')",
  "const { query: renamed } = db; renamed('SELECT id FROM repositories')",
  'db[method](query)',
  'const statement = `SELECT id FROM ${table}`',
  "const statement = 'INSERT INTO repositories (id) VALUES (?)'",
  "const statement = 'PRAGMA foreign_keys = OFF'",
  'sql`SELECT id FROM repositories`',
  'db.select().from(table)',
  'db.transaction(() => {})',
  'new GraphStore(db).view(id, scope)',
  'createRepositoryContext()',
  'const alias = repository; alias.view(id, scope)',
  'repository.list()',
  'db.sql(statement)',
  "new Function('return database')()",
]
for (const example of failures)
  test(`rejects consumer bypass: ${example}`, () => {
    expect(auditSource('src/routes/bypass.ts', example).length).toBeGreaterThan(
      0,
    )
  })

test('bundler exclusion strings and domain-only service/DTO imports are permitted', () => {
  expect(
    auditSource(
      'vite.config.ts',
      "export default { optimizeDeps: { exclude: ['bun', 'bun:sqlite'] } }",
    ),
  ).toEqual([])
  expect(
    auditSource(
      'src/routes/consumer.ts',
      "import { createRepositoryService } from '@/lib/repositories.servers'; import type { Repository } from '@/lib/api'",
    ),
  ).toEqual([])
})

test('local forwarding imports fail at the bridge even when route uses an innocuous alias', () => {
  const files = {
    'src/routes/consumer.ts': "import { safe } from '../bridge'; safe()",
    'src/bridge.ts':
      "import { RepositoryStore as Safe } from './lib/persistence/repositories/repository-store'; export const safe = Safe",
  }
  expect(
    Object.entries(files).flatMap(([file, source]) =>
      auditSource(file, source),
    ),
  ).toContainEqual(expect.stringContaining('src/bridge.ts'))
})

test('raw SQL is forbidden inside repositories too; infrastructure exception is exact', () => {
  for (const source of [
    "import { sql as text } from 'drizzle-orm'; text`select * from repositories`",
    "import * as orm from 'drizzle-orm'; orm.sql.raw(query)",
    "db.query('SELECT id FROM repositories')",
    'db.execute(query)',
    "import { Database } from 'bun:sqlite'",
  ])
    expect(
      auditSource(`${repositoryRoot}graph-store.ts`, source).length,
    ).toBeGreaterThan(0)
  const infrastructure = `${repositoryRoot}connection.ts`
  expect(
    auditSource(infrastructure, "driver.run('PRAGMA foreign_keys = ON')"),
  ).toEqual([])
  expect(
    auditSource(infrastructure, "driver.run('SELECT id FROM repositories')")
      .length,
  ).toBeGreaterThan(0)
  expect(
    auditSource(infrastructure, "driver.run('PRAGMA journal_mode = DELETE')")
      .length,
  ).toBeGreaterThan(0)
  expect(
    auditSource(infrastructure, "other.run('PRAGMA foreign_keys = ON')").length,
  ).toBeGreaterThan(0)
  expect(
    auditSource(infrastructure, 'driver.run(query)').length,
  ).toBeGreaterThan(0)
  expect(
    auditSource(infrastructure, 'const { run: execute } = driver').length,
  ).toBeGreaterThan(0)
  expect(
    auditSource(
      serviceFile,
      "export { GraphStore } from './persistence/repositories/graph-store'",
    ).length,
  ).toBeGreaterThan(0)
})

function serviceProgram(source: string) {
  const file = join(root, serviceFile)
  const config = ts.readConfigFile(join(root, 'tsconfig.json'), ts.sys.readFile)
  const options = ts.parseJsonConfigFileContent(
    config.config,
    ts.sys,
    root,
  ).options
  const host = ts.createCompilerHost(options)
  const original = host.getSourceFile.bind(host)
  host.getSourceFile = (...args: Parameters<typeof original>) =>
    args[0] === file
      ? ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true)
      : original(...args)
  const program = ts.createProgram([file], options, host)
  return { program, file: program.getSourceFile(file)! }
}
test('inferred service handle aliases and opaque exports are rejected', () => {
  for (const source of [
    'export const leaked: any = null',
    'export const factory = () => ({ handle: null as any })',
    'export type Hidden = any',
  ]) {
    const { program, file } = serviceProgram(source)
    expect(auditServiceExports(program, file).length).toBeGreaterThan(0)
  }
})
