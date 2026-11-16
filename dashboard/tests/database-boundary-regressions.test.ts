import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, test } from 'bun:test'
import ts from 'typescript-ast'
import {
  auditServiceExports,
  auditSource,
  repositoryRoot,
  serviceFile,
} from './database-architecture'
import { auditRepositoryDependencies } from './database-dependency-policy'

// These integration tests compile fixture programs or the full dashboard on CI.
const compilerTimeout = 30_000
const root = fileURLToPath(new URL('..', import.meta.url)).replace(/\/$/, '')
const config = ts.readConfigFile(join(root, 'tsconfig.json'), ts.sys.readFile)
const options = ts.parseJsonConfigFileContent(
  config.config,
  ts.sys,
  root,
).options

// Fixture sources stay fresh; unchanged dependencies can reuse their parsed ASTs.
const dependencySources = new Map<string, ts.SourceFile>()

function fixtureProgram(fixtures: Record<string, string>) {
  const files = new Map(
    Object.entries({
      // Model an accidental bootstrap leak, not the safe composed API returned in production.
      [`${repositoryRoot}factory.ts`]: `import type { RepositoryRepositoryPort } from '../../repository-ports'; export const createRepositoryContext = (): RepositoryRepositoryPort => null!`,
      ...fixtures,
    }).map(([file, text]) => [join(root, file), text]),
  )
  const host = ts.createCompilerHost(options)
  const original = host.getSourceFile.bind(host)
  host.getSourceFile = (...args: Parameters<typeof original>) => {
    const text = files.get(args[0])
    if (text !== undefined)
      return ts.createSourceFile(args[0], text, ts.ScriptTarget.Latest, true)
    const cached = dependencySources.get(args[0])
    if (cached && !args[3]) return cached
    const source = original(...args)
    if (source) dependencySources.set(args[0], source)
    return source
  }
  const originalExists = host.fileExists.bind(host)
  const originalRead = host.readFile.bind(host)
  host.fileExists = (name) => files.has(name) || originalExists(name)
  host.readFile = (name) => files.get(name) ?? originalRead(name)
  return ts.createProgram([...files.keys()], options, host)
}

function dependencyErrors(fixtures: Record<string, string>) {
  const program = fixtureProgram(fixtures)
  const sources = Object.keys(fixtures).map((file) =>
    program.getSourceFile(join(root, file))!,
  )
  expect(
    sources
      .flatMap((source) => [
        ...program.getSyntacticDiagnostics(source),
        ...program.getSemanticDiagnostics(source),
      ])
      .map((diagnostic) =>
        ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n'),
      ),
  ).toEqual([])
  return sources.flatMap((source) =>
    auditRepositoryDependencies(program, source),
  )
}

for (const source of [
  "import { eq, StringChunk } from 'drizzle-orm'; const condition = eq(repositories.id, id); condition.queryChunks.push(new StringChunk(' OR 1=1'))",
  "import { StringChunk as Fragment } from 'drizzle-orm'; const Chunk = Fragment; new Chunk(fragment)",
  "import type { SQLWrapper as Wrapper } from 'drizzle-orm'",
  "type Wrapper = import('drizzle-orm').SQLWrapper",
  "export { StringChunk as Fragment } from 'drizzle-orm/sql/sql'",
  "export * from 'drizzle-orm/sql/sql'",
  "import { SQL as Query } from 'drizzle-orm/sql/sql'; new Query(chunks)",
  'condition.queryChunks[0] = fragment',
  'const { queryChunks: chunks } = condition; chunks.push(fragment)',
  "condition['queryChunks'].push(fragment)",
  'condition.append(fragment)',
  'condition.inlineParams()',
  'const wrapper = { getSQL() { return fragment } }',
  "const wrapper = { ['getSQL']() { return fragment } }",
  'const wrapper = { getSQL: () => fragment }',
  'class Wrapper { getSQL() { return fragment } }',
])
  test(`rejects Drizzle raw chunk/wrapper escape: ${source}`, () => {
    expect(
      auditSource(`${repositoryRoot}graph-queries.ts`, source).length,
    ).toBeGreaterThan(0)
  })

test('connection infrastructure cannot run mapped domain queries', () => {
  for (const source of [
    'database.select().from(repositories)',
    'database.delete(repositories)',
    'database.insert(repositories)',
  ])
    expect(
      auditSource(`${repositoryRoot}connection.ts`, source).length,
    ).toBeGreaterThan(0)
})

for (const source of [
  "import type { RepositoryRepositoryPort as Port } from './lib/repository-ports'; export function outside(r: Port) { return r.list() }",
  "export type { RepositoryRepositoryPort as PublicPort } from './lib/repository-ports'",
  "type Alias = import('./lib/repository-ports').RepositoryRepositoryPort",
  "import * as ports from './lib/repository-ports'; export type Alias = ports.RepositoryRepositoryPort",
])
  test(`rejects repository-port import/forwarding outside owners: ${source}`, () => {
    expect(auditSource('src/bridge.ts', source).length).toBeGreaterThan(0)
  })

test(
  'typed port receivers, aliases and constructor fields are guarded regardless of their names',
  () => {
    for (const text of [
      'export function outside(r: Port) { return r.list() }',
      'export function outside(r: Port) { const alias = r; return alias.list() }',
      'export class Outside { constructor(private readonly r: Port) {} list() { return this.r.list() } }',
      "type Alias = Pick<Port, 'list'>; export function outside(r: Alias) { return r.list() }",
      "type Alias = Port['list']; export function outside(read: Alias) { return read() }",
    ]) {
      const errors = dependencyErrors({
        'src/boundary-helper.ts': `import type { RepositoryRepositoryPort as Port } from './lib/repository-ports'; ${text}`,
      })
      expect(errors.some((error) => error.includes('dependency outside'))).toBe(
        true,
      )
    }
  },
  compilerTimeout,
)

const structuralPort = `type Port = {
  register(input: { id: string; name: string; location: string; at: number }): unknown
  list(): unknown[]
  view(id: string, scope: unknown): unknown
  close(): void
}`

test('structural port aliases cannot evade dependency enforcement', () => {
  expect(
    dependencyErrors({
      'src/boundary-helper.ts': `${structuralPort}; export function outside(r: Port) { const x = r; return x.list() }`,
    }).length,
  ).toBeGreaterThan(0)
})

test(
  'service cannot delegate inferred port dependencies to nonservice helpers or constructors',
  () => {
    for (const [helper, call] of [
      [
        'export function outside<T>(r: T) { return r }',
        'outside(createRepositoryContext())',
      ],
      [
        'export class Outside<T> { constructor(public readonly r: T) {} }',
        'new Outside(createRepositoryContext())',
      ],
      [
        'export function outside(r: unknown) { return r }',
        'outside(createRepositoryContext() as unknown)',
      ],
      [
        'export function outside(r: unknown) { return r }',
        'outside({ hidden: createRepositoryContext() })',
      ],
    ]) {
      const errors = dependencyErrors({
        'src/boundary-helper.ts': helper,
        [serviceFile]: `import { createRepositoryContext } from './persistence/repositories/factory'; import { ${call.startsWith('new') ? 'Outside' : 'outside'} } from '../boundary-helper'; export const operation = () => ${call}`,
      })
      expect(errors.some((error) => error.includes('delegated outside'))).toBe(
        true,
      )
    }
  },
  compilerTimeout,
)

test(
  'DTO-returning helpers cannot receive ports, including erased aliases',
  () => {
    for (const setup of [
      'const r = createRepositoryContext()',
      'const r: unknown = createRepositoryContext()',
      'const masked: any = createRepositoryContext(); const r = masked',
      'let r: unknown; r = createRepositoryContext()',
      'class Holder { r: unknown; constructor() { this.r = createRepositoryContext() } }; const holder = new Holder(); const r = holder.r',
    ]) {
      const fixtures = {
        'src/boundary-helper.ts':
          "import type { RepositoryRepositoryPort as P } from './lib/repository-ports'; import type { RepositoryRecord } from './lib/graph-records'; export const outside = (r: unknown): RepositoryRecord[] => (r as P).list()",
        [serviceFile]: `import { createRepositoryContext } from './persistence/repositories/factory'; import { outside as read } from '../boundary-helper'; export function operation() { ${setup}; return read(r) }`,
      }
      const program = fixtureProgram(fixtures)
      const service = program.getSourceFile(join(root, serviceFile))!
      expect(program.getSemanticDiagnostics(service)).toEqual([])
      expect(auditSource(serviceFile, fixtures[serviceFile])).toEqual([])
      expect(auditServiceExports(program, service)).toEqual([])
      expect(
        auditRepositoryDependencies(program, service).some((error) =>
          error.includes('delegated outside'),
        ),
      ).toBe(true)
    }
  },
  compilerTimeout,
)

test('service-private helpers and domain DTO handoff remain permitted', () => {
  expect(
    dependencyErrors({
      'src/lib/services/repository-service.ts': `import type { RepositoryRepositoryPort } from '../repository-ports'; import { outside } from '../../boundary-helper'; function inside(r: RepositoryRepositoryPort) { return outside(r.list()) }; export const operation = (r: RepositoryRepositoryPort) => inside(r)`,
      'src/boundary-helper.ts':
        "import type { RepositoryRecord } from './lib/graph-records'; export const outside = (rows: RepositoryRecord[]) => rows.length",
    }),
  ).toEqual([])
})

test('a capability cannot pass its repository to a different capability, even through unknown', () => {
  const errors = dependencyErrors({
    'src/lib/services/repository-service.ts': `
      import type { RepositoryRepositoryPort } from '../repository-ports'
      import { outside } from './node-service'
      export function read(repository: RepositoryRepositoryPort) {
        const erased: unknown = repository
        return outside(erased)
      }
    `,
    'src/lib/services/node-service.ts':
      'export function outside(value: unknown) { return value }',
  })
  expect(errors.some((error) => error.includes('delegated outside'))).toBe(true)
})

test('composition may inject repositories but cannot call their operations', () => {
  const errors = dependencyErrors({
    [`${repositoryRoot}factory.ts`]: `
      import { NodeRepository } from './node-repository'
      const repository = new NodeRepository(null as any)
      export const bypass = () => repository.read('id')
    `,
  })
  expect(
    errors.some((error) => error.includes('composition may construct')),
  ).toBe(true)
})

test('composition cannot forward raw database handles to application helpers', () => {
  for (const setup of [
    'const value = openConnection().database',
    'const value: unknown = openConnection().database',
    'const value = { hidden: openConnection().database }',
  ]) {
    const errors = dependencyErrors({
      [`${repositoryRoot}factory.ts`]: `
        import { openConnection } from './connection'
        import { outside } from '../../../boundary-helper'
        export function createRepositoryContext() {
          ${setup}
          return outside(value)
        }
      `,
      'src/boundary-helper.ts':
        'export function outside(value: unknown) { return value }',
    })
    expect(errors.some((error) => error.includes('delegated outside'))).toBe(
      true,
    )
  }
})

test(
  'all production sources enforce typed repository dependency ownership',
  () => {
    const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, root)
    const program = ts.createProgram(parsed.fileNames, options)
    const sources = program
      .getSourceFiles()
      .filter(
        (source) =>
          source.fileName.startsWith(`${root}/`) &&
          !source.fileName.includes('/node_modules/') &&
          !source.fileName.includes('/tests/') &&
          !/\.(?:test|spec)\./.test(source.fileName),
      )
    expect(sources.length).toBeGreaterThan(0)
    expect(
      sources.flatMap((source) => auditRepositoryDependencies(program, source)),
    ).toEqual([])
  },
  compilerTimeout,
)
