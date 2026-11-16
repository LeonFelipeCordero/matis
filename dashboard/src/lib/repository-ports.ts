import type {
  DecoratorRecord,
  DiagnosticRecord,
  FileRecord,
  GraphRevisionRecord,
  NodeMetricsRecord,
  NodeRecord,
  RelationshipRecord,
  RepositoryRecord,
  SymbolRecord,
} from './graph-records'

export type RepositoryRepositoryPort = {
  register: (input: RepositoryRecord) => RepositoryRecord | undefined
  find: (id: string) => RepositoryRecord | undefined
  list: () => Array<RepositoryRecord>
}
export type GraphRevisionRepositoryPort = {
  find: (id: string) => GraphRevisionRecord | undefined
  read: (ids: Array<string>) => Array<GraphRevisionRecord>
}
export type NodeRepositoryPort = { read: (id: string) => Array<NodeRecord> }
export type FileRepositoryPort = {
  read: (id: string, keys: Array<string>) => Array<FileRecord>
}
export type SymbolRepositoryPort = {
  read: (id: string, keys?: Array<string>) => Array<SymbolRecord>
}
export type DecoratorRepositoryPort = {
  read: (id: string, keys: Array<string>) => Array<DecoratorRecord>
}
export type NodeMetricsRepositoryPort = {
  read: (id: string, keys: Array<string>) => Array<NodeMetricsRecord>
}
export type RelationshipRepositoryPort = {
  outgoing: (id: string, keys?: Array<string>) => Array<RelationshipRecord>
  relations: (
    id: string,
  ) => Array<Pick<RelationshipRecord, 'sourceStableKey' | 'targetStableKey'>>
  imports: (id: string) => Array<RelationshipRecord>
}
export type DiagnosticRepositoryPort = {
  read: (id: string) => Array<DiagnosticRecord>
}
