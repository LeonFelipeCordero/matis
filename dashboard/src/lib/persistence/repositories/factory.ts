import { RepositoryService } from '../../services/repository-service'
import { GraphRevisionService } from '../../services/graph-revision-service'
import { NodeService } from '../../services/node-service'
import { RelationshipService } from '../../services/relationship-service'
import { NodeMetricsService } from '../../services/node-metrics-service'
import { DiagnosticService } from '../../services/diagnostic-service'
import { graphView } from '../../services/graph-view'
import { defaultDatabasePath, openConnection } from './connection'
import { RepositoryStore } from './repository-store'
import { GraphRevisionRepository } from './graph-revision-repository'
import { NodeRepository } from './node-repository'
import { FileRepository } from './file-repository'
import { SymbolRepository } from './symbol-repository'
import { DecoratorRepository } from './decorator-repository'
import { RelationshipRepository } from './relationship-repository'
import { NodeMetricsRepository } from './node-metrics-repository'
import { DiagnosticRepository } from './diagnostic-repository'
import type { AppDatabase } from './connection'
import type { GraphScope } from '../../project-graph-types'

function capabilities(database: AppDatabase) {
  const revisions = new GraphRevisionService(
    new GraphRevisionRepository(database),
  )
  return {
    repositories: new RepositoryService(
      new RepositoryStore(database),
      revisions,
    ),
    revisions,
    nodes: new NodeService(
      new NodeRepository(database),
      new FileRepository(database),
      new SymbolRepository(database),
      new DecoratorRepository(database),
    ),
    relationships: new RelationshipService(
      new RelationshipRepository(database),
    ),
    metrics: new NodeMetricsService(new NodeMetricsRepository(database)),
    diagnostics: new DiagnosticService(new DiagnosticRepository(database)),
  }
}

/** Composition only; each graph/list read uses one explicit deferred WAL snapshot. */
export function createRepositoryContext(path?: string) {
  const connection = openConnection(path)
  const services = capabilities(connection.database)
  return {
    registerRepository: (input: unknown) =>
      services.repositories.register(input),
    listRepositories: () =>
      connection.database.transaction(
        (transaction) => capabilities(transaction).repositories.list(),
        { behavior: 'deferred' },
      ),
    getArchitecture: (id: string, scope: GraphScope) =>
      connection.database.transaction(
        (transaction) => graphView(id, scope, capabilities(transaction)),
        { behavior: 'deferred' },
      ),
    close: () => connection.close(),
  }
}

export const repositoryDatabasePath = () => defaultDatabasePath()
