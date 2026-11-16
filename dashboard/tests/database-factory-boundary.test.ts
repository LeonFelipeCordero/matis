import { fileURLToPath } from 'node:url'
import { join } from 'node:path'
import { expect, test } from 'bun:test'
import ts from 'typescript-ast'
import {
  auditServiceExports,
  auditSource,
  serviceFile,
} from './database-architecture'

const root = fileURLToPath(new URL('..', import.meta.url))
test('service cannot import connection, stores, schema or ORM, even with aliases/types/dynamic loading', () => {
  for (const source of [
    "import { openConnection as context } from './persistence/repositories/connection'",
    "import type { AppDatabase as DTO } from './persistence/repositories/connection'",
    "import { GraphStore as Port } from './persistence/repositories/graph-store'",
    "import { graphNodes } from './persistence/repositories/schema'",
    "import * as context from './persistence/repositories/factory'",
    "const context = await import('./persistence/repositories/factory')",
    "export { createRepositoryContext as factory } from './persistence/repositories/factory'",
    "import type { BunSQLiteDatabase } from 'drizzle-orm/bun-sqlite'",
    'const context = openConnection()',
  ])
    expect(auditSource(serviceFile, source).length).toBeGreaterThan(0)
  expect(
    auditSource(
      serviceFile,
      "import { createRepositoryContext as context } from './persistence/repositories/factory'",
    ),
  ).toEqual([])
})

test('inferred service exports cannot forward factory, context, class, ORM or connection handles', () => {
  const source = `
    import { createRepositoryContext as factory } from './persistence/repositories/factory'
    import { openConnection } from './persistence/repositories/connection'
    import { RepositoryStore } from './persistence/repositories/repository-store'
    export const leakedFactory = factory
    export const leakedContext = () => factory()
    export const leakedClass = RepositoryStore
    export const leakedConnection = () => openConnection()
    export const leakedOrm = () => openConnection().database
    export const nestedContext = () => ({ hidden: factory() })
    export const arrayContext = () => [factory()]
    export const promisedContext = async () => factory()
    export const mappedContext = () => new Map([['private', factory()]])
  `
  const file = join(root, serviceFile)
  const config = ts.readConfigFile(join(root, 'tsconfig.json'), ts.sys.readFile)
  const options = ts.parseJsonConfigFileContent(
    config.config,
    ts.sys,
    root,
  ).options
  const host = ts.createCompilerHost(options)
  const factoryFile = join(root, 'src/lib/persistence/repositories/factory.ts')
  const leakedFactory = `import type { RepositoryRepositoryPort } from '../../repository-ports'; export const createRepositoryContext = (): RepositoryRepositoryPort => null!`
  const original = host.getSourceFile.bind(host)
  host.getSourceFile = (...args: Parameters<typeof original>) =>
    args[0] === file
      ? ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true)
      : args[0] === factoryFile
        ? ts.createSourceFile(
            factoryFile,
            leakedFactory,
            ts.ScriptTarget.Latest,
            true,
          )
        : original(...args)
  const program = ts.createProgram([file], options, host)
  const errors = auditServiceExports(program, program.getSourceFile(file)!)
  for (const name of [
    'leakedFactory',
    'leakedContext',
    'leakedClass',
    'leakedConnection',
    'leakedOrm',
    'nestedContext',
    'arrayContext',
    'promisedContext',
    'mappedContext',
  ])
    expect(errors.some((error) => error.startsWith(`${name}:`))).toBe(true)
})
