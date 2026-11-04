import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, expect, spyOn, test } from 'bun:test'
import { Database } from 'bun:sqlite'
import { rustCli } from '../../tests/rust-cli'
import { migratedDatabaseFixture } from '../../tests/migrated-database-fixture'
import {
  createRepositoryService,
  repositoryErrorResponse,
} from './repositories.servers'
import type { RepositoryService } from './repositories.servers'
import type { GraphView } from './graph-data'
import type { GraphScope } from './project-graph-types'

let directory: string
let path: string
let database: Database
let service: RepositoryService
const copyMigratedDatabase = migratedDatabaseFixture()

// Raw connections belong to test fixtures/assertions only, never the service API.
const openDatabase = (file: string) =>
  new Database(file, { readwrite: true, create: false, strict: true })
const registerRepository = (input: unknown, _fixture: Database) =>
  service.registerRepository(input)
const listRepositories = (_fixture: Database) => service.listRepositories()
const getArchitecture = (id: string, _fixture: Database, scope?: GraphScope) =>
  service.getArchitecture(id, scope)

function ready(
  id: string,
  scope: GraphScope = { kind: 'repository' },
): GraphView {
  const result = getArchitecture(id, database, scope)
  expect(result?.state).toBe('ready')
  if (result?.state !== 'ready') throw new Error('Expected normalized graph')
  expect(result).not.toHaveProperty('artifact')
  return result
}
const fileView = (id: string, key = 'file:main.py') =>
  ready(id, { kind: 'file', key })

const matis = (...args: Array<string>) => rustCli(path, ...args)

beforeEach(() => {
  directory = mkdtempSync(join(tmpdir(), 'shared-graph-database-'))
  path = join(directory, 'nested', 'test.sqlite')
  copyMigratedDatabase(path)
  database = openDatabase(path)
  service = createRepositoryService(path)
})

afterEach(() => {
  service.close()
  database.close()
  rmSync(directory, { recursive: true, force: true })
})

test('UI opens a Rust-migrated database without owning migrations', () => {
  const repository = registerRepository(
    { name: ' Example ', location: ' /work/example ' },
    database,
  )
  expect(repository.name).toBe('Example')
  expect(repository.location).toBe('/work/example')
  expect(repository.createdAt).toBeGreaterThan(1_000_000_000_000)
  expect(repository.importedAt).toBeNull()
  expect(listRepositories(database)).toEqual([repository])
  expect(getArchitecture(repository.id, database)).toBeNull()
  service.close()
  service = createRepositoryService(path)
  expect(listRepositories(database)).toEqual([repository])
  expect(
    database.query('SELECT version_num FROM alembic_version').get(),
  ).toEqual({ version_num: '0005_mutation_candidates' })
  expect(
    database
      .query("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
      .all(),
  ).toEqual([
    { name: 'alembic_version' },
    { name: 'graph_diagnostics' },
    { name: 'graph_edges' },
    { name: 'graph_files' },
    { name: 'graph_mutation_candidates' },
    { name: 'graph_node_decorators' },
    { name: 'graph_node_metrics' },
    { name: 'graph_nodes' },
    { name: 'graph_symbols' },
    { name: 'repositories' },
    { name: 'repository_graphs' },
    { name: 'seaql_migrations' },
  ])
})

test('missing or unmigrated databases fail without creating a schema', () => {
  const missing = join(directory, 'missing.sqlite')
  expect(() => createRepositoryService(missing)).toThrow()
  expect(existsSync(missing)).toBe(false)
  const emptyPath = join(directory, 'empty.sqlite')
  const empty = new Database(emptyPath, { create: true })
  empty.close()
  const check = openDatabase(emptyPath)
  const emptyService = createRepositoryService(emptyPath)
  expect(() => emptyService.listRepositories()).toThrow('no such table')
  emptyService.close()
  expect(
    check.query('SELECT name FROM sqlite_master WHERE type = ?').all('table'),
  ).toEqual([])
  check.close()
})

test('name uniqueness and validation retain existing API behavior', () => {
  registerRepository({ name: 'Example', location: 'local' }, database)
  expect(() =>
    registerRepository({ name: 'EXAMPLE', location: 'other' }, database),
  ).toThrow('already exists')
  for (const input of [
    null,
    [],
    {},
    { name: 5, location: 'x' },
    { name: ' ', location: 'x' },
    { name: 'x', location: ' ' },
    { name: 'x'.repeat(201), location: 'x' },
  ])
    expect(() => registerRepository(input, database)).toThrow()
  expect(listRepositories(database)).toHaveLength(1)
  expect(() => getArchitecture('unknown', database)).toThrow(
    'Repository not found',
  )
  expect(() => getArchitecture(' ', database)).toThrow(
    'repositoryId is required',
  )
})

test('Rust indexes repositories registered by the UI, and the UI sees reindex writes', () => {
  const source = join(directory, 'source')
  mkdirSync(source)
  const file = join(source, 'main.py')
  writeFileSync(file, 'def run():\n    return 1\n')
  const repository = registerRepository(
    { name: 'Shared', location: source },
    database,
  )
  const summary = JSON.parse(matis('index', '-f', source))
  const first = fileView(repository.id)
  expect(summary.repositoryId).toBe(repository.id)
  expect(first.graph.nodes.some((node) => node.label === 'run')).toBe(true)
  writeFileSync(file, 'def replacement():\n    return 2\n')
  matis('index', '-f', source)
  const replacement = fileView(repository.id)
  expect(
    replacement.graph.nodes.some((node) => node.label === 'replacement'),
  ).toBe(true)
  expect(replacement.graph.nodes.some((node) => node.label === 'run')).toBe(
    false,
  )
  expect(replacement.graphRevision).not.toBe(first.graphRevision)
  expect(listRepositories(database)[0].importedAt).toBe(replacement.importedAt)
  expect(
    database.query('SELECT count(*) AS count FROM repository_graphs').get(),
  ).toEqual({ count: 1 })
  expect(existsSync(join(source, 'graph.json'))).toBe(false)
  expect(() => getArchitecture(repository.name, database)).toThrow(
    'Repository not found',
  )

  // API lookups must still use IDs, even when a name resembles another ID.
  const otherSource = join(directory, 'other-source')
  mkdirSync(otherSource)
  const idNamed = registerRepository(
    { name: repository.id, location: otherSource },
    database,
  )
  writeFileSync(join(otherSource, 'main.py'), 'def id_named():\n    return 3\n')
  const selected = JSON.parse(matis('index', '-f', otherSource))
  expect(selected.repositoryId).toBe(idNamed.id)
  const idNamedGraph = fileView(idNamed.id)
  expect(
    idNamedGraph.graph.nodes.some((node) => node.label === 'id_named'),
  ).toBe(true)
  expect(fileView(repository.id)).toEqual(replacement)
  expect(
    database
      .query(
        'SELECT repository_id FROM repository_graphs ORDER BY repository_id',
      )
      .all(),
  ).toEqual(
    [repository.id, idNamed.id]
      .sort()
      .map((repository_id) => ({ repository_id })),
  )
})

test('folder reports register and update graphs visible to the UI', () => {
  const source = join(directory, 'source')
  mkdirSync(source)
  const file = join(source, 'main.py')
  writeFileSync(file, 'def first():\n    return 1\n')
  matis('report', '-f', source, '--save')
  const repository = listRepositories(database)[0]
  expect(repository.name).toBe('source')
  expect(repository.location).toBe(source)
  expect(
    fileView(repository.id).graph.nodes.some((node) => node.label === 'first'),
  ).toBe(true)
  writeFileSync(file, 'def replacement():\n    return 2\n')
  matis('report', '--f', source, '--save')
  expect(listRepositories(database)).toHaveLength(1)
  expect(
    fileView(repository.id).graph.nodes.some(
      (node) => node.label === 'replacement',
    ),
  ).toBe(true)
})

test('Python src imports become dashboard arrows even when the report has no findings', () => {
  const source = join(directory, 'python-source')
  const app = join(source, 'src', 'app')
  mkdirSync(app, { recursive: true })
  writeFileSync(join(app, '__init__.py'), '')
  writeFileSync(
    join(app, 'main.py'),
    'from app import worker as w\ndef run():\n    w.work()\n',
  )
  writeFileSync(join(app, 'worker.py'), 'def work(): pass\n')
  matis('report', '-f', source, '--save')
  const repository = listRepositories(database)[0]
  const { graph } = ready(repository.id, {
    kind: 'folder',
    key: 'module:src/app',
  })
  expect(graph.edges).toHaveLength(1)
  expect(graph.edges[0]).toMatchObject({
    source: 'file:src/app/main.py',
    target: 'file:src/app/worker.py',
    relationships: { IMPORTS: 1, CALLS: 1 },
  })
})

test('indexed metrics survive Rust/Bun storage and keep hidden test dependents', () => {
  const source = join(directory, 'impact-source')
  mkdirSync(source)
  writeFileSync(join(source, 'core.py'), 'def core():\n    return 1\n')
  writeFileSync(
    join(source, 'main.py'),
    'from core import core\ndef main():\n    return core()\n',
  )
  writeFileSync(
    join(source, 'test_main.py'),
    'from main import main\ndef test_main():\n    return main()\n',
  )
  matis('report', '-f', source, '--save')
  const repository = listRepositories(database)[0]
  const view = ready(repository.id)
  const core = view.details.nodes.find((node) => node.name === 'core.py')!
  expect(core.properties).toMatchObject({
    fanIn: 1,
    fanOut: 0,
    instability: 0,
    transitiveChangeImpact: 2,
  })
  expect(view.details.nodes.some((node) => node.name === 'test_main.py')).toBe(
    false,
  )
  const { graph } = view
  expect(
    graph.edges.filter((edge) => edge.target === core.stableKey),
  ).toHaveLength(1)
})

test('legacy rows require reindex and never look like valid empty graphs', () => {
  const repository = registerRepository(
    { name: 'Legacy', location: 'local' },
    database,
  )
  database
    .query(
      'INSERT INTO repository_graphs (repository_id, imported_at) VALUES (?, ?)',
    )
    .run(repository.id, 123)
  expect(getArchitecture(repository.id, database)).toEqual({
    state: 'needsReindex',
    repositoryId: repository.id,
    importedAt: 123,
    rootName: 'Legacy',
  })
  expect(listRepositories(database)[0].importedAt).toBeNull()
  expect(
    database
      .query<{ name: string }, []>('PRAGMA table_info(repository_graphs)')
      .all()
      .map((column) => column.name),
  ).not.toContain('artifact')
})

test('one read transaction keeps metadata and nodes on one WAL snapshot', () => {
  const source = join(directory, 'source')
  mkdirSync(source)
  const file = join(source, 'main.py')
  writeFileSync(file, 'def before(): return 1\n')
  matis('index', '-f', source)
  const repository = listRepositories(database)[0]
  const before = fileView(repository.id)
  writeFileSync(file, 'def after(): return 2\n')
  const original = Database.prototype.prepare
  let interleaved = false
  let connection: Database | undefined
  const spy = spyOn(Database.prototype, 'prepare').mockImplementation(function (
    this: Database,
    sql: string,
  ) {
    expect(this.inTransaction).toBe(true)
    connection = this
    if (
      !interleaved &&
      sql.includes('from "graph_nodes" left join "graph_symbols"')
    ) {
      interleaved = true
      matis('index', '-f', source)
    }
    return original.call(this, sql)
  } as typeof original)
  try {
    expect(fileView(repository.id)).toEqual(before)
    expect(interleaved).toBe(true)
  } finally {
    spy.mockRestore()
  }
  const after = fileView(repository.id)
  expect(after.graphRevision).not.toBe(before.graphRevision)
  expect(after.graph.nodes.map((node) => node.label)).toEqual(['after'])
  expect(connection?.inTransaction).toBe(false)
})

test('error responses remain JSON with the expected status', async () => {
  let error: unknown
  try {
    getArchitecture('unknown', database)
  } catch (caught) {
    error = caught
  }
  const response = repositoryErrorResponse(error)
  expect(response.status).toBe(404)
  expect(await response.json()).toEqual({ error: 'Repository not found' })
})
