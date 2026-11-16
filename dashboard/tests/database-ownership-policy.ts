import ts from 'typescript-ast'
import { factory, repositoryRoot } from './database-policy'
import type { Reporter } from './database-syntax'

const serviceRoot = 'src/lib/services/'
export const ownership = [
  [
    'repositories',
    'repository-store',
    'RepositoryRepositoryPort',
    'repository-service',
  ],
  [
    'repositoryGraphs',
    'graph-revision-repository',
    'GraphRevisionRepositoryPort',
    'graph-revision-service',
  ],
  ['graphNodes', 'node-repository', 'NodeRepositoryPort', 'node-service'],
  ['graphFiles', 'file-repository', 'FileRepositoryPort', 'node-service'],
  ['graphSymbols', 'symbol-repository', 'SymbolRepositoryPort', 'node-service'],
  [
    'graphDecorators',
    'decorator-repository',
    'DecoratorRepositoryPort',
    'node-service',
  ],
  [
    'graphEdges',
    'relationship-repository',
    'RelationshipRepositoryPort',
    'relationship-service',
  ],
  [
    'graphMetrics',
    'node-metrics-repository',
    'NodeMetricsRepositoryPort',
    'node-metrics-service',
  ],
  [
    'graphDiagnostics',
    'diagnostic-repository',
    'DiagnosticRepositoryPort',
    'diagnostic-service',
  ],
] as const

export const capabilityFiles = new Set(
  ownership.map(([, , , owner]) => `${serviceRoot}${owner}.ts`),
)
export function permittedPorts(file: string) {
  return new Set<string>(
    ownership
      .filter(
        ([, repository, , owner]) =>
          file.endsWith(`/${repositoryRoot}${repository}.ts`) ||
          file === `${repositoryRoot}${repository}.ts` ||
          file.endsWith(`/${serviceRoot}${owner}.ts`) ||
          file === `${serviceRoot}${owner}.ts`,
      )
      .map(([, , port]) => port),
  )
}
export function repositoryPort(file: string) {
  return ownership.find(
    ([, repository]) =>
      file.endsWith(`/${repositoryRoot}${repository}.ts`) ||
      file === `${repositoryRoot}${repository}.ts`,
  )?.[2]
}

function namedImports(node: ts.Node) {
  const declaration = node.parent
  if (!ts.isImportDeclaration(declaration) || declaration.importClause?.name)
    return undefined
  const named = declaration.importClause?.namedBindings
  return named && ts.isNamedImports(named) ? named.elements : undefined
}

export function checkTableOwnership(
  node: ts.Node,
  file: string,
  resolved: string,
  report: Reporter,
) {
  if (
    file.startsWith(repositoryRoot) &&
    file !== `${factory}.ts` &&
    resolved.startsWith(serviceRoot)
  )
    report(node, 'repository cannot depend on a capability service')
  if (resolved === `${repositoryRoot}schema`) {
    const names = namedImports(node)
    if (
      !names?.length ||
      names.some(
        (binding) =>
          !ownership.some(
            ([table, repository]) =>
              table === (binding.propertyName ?? binding.name).text &&
              file === `${repositoryRoot}${repository}.ts`,
          ),
      )
    )
      report(node, 'table import outside its sole repository')
  }
  if (resolved === 'src/lib/repository-ports') {
    const names = namedImports(node)
    const ports = permittedPorts(file)
    if (
      !names?.length ||
      names.some(
        (binding) => !ports.has((binding.propertyName ?? binding.name).text),
      )
    )
      report(node, 'port import outside its owning capability/repository')
  }
  if (
    resolved.startsWith(repositoryRoot) &&
    file.startsWith(repositoryRoot) &&
    file !== `${factory}.ts` &&
    ![
      `${repositoryRoot}schema`,
      `${repositoryRoot}connection`,
      `${repositoryRoot}selected-rows`,
    ].includes(resolved)
  )
    report(node, 'repository cannot delegate to another repository')
}
