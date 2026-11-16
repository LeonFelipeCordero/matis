import type { Repository } from './api'
import type {
  ArtifactNode,
  DirectEdge,
  NodeComplexity,
  NodeSizeMetrics,
} from './architecture-artifact'

export type RepositoryRecord = Omit<Repository, 'importedAt'>
export type GraphRevisionRecord = {
  repositoryId: string
  importedAt: number
  graphRevision: string | null
  rootName: string | null
}
export type NodeRecord = Pick<
  ArtifactNode,
  | 'stableKey'
  | 'parentStableKey'
  | 'type'
  | 'kind'
  | 'level'
  | 'name'
  | 'language'
  | 'qualifiedName'
  | 'path'
>
export type FileRecord = {
  stableKey: string
  bytes: number | null
  sha256: string | null
}
export type SymbolRecord = {
  stableKey: string
  startLine: number | null
  endLine: number | null
  startColumn: number | null
  endColumn: number | null
  signature: string | null
  visibility: string | null
}
export type DecoratorRecord = { stableKey: string; decorator: string }
export type NodeMetricsRecord = { stableKey: string } & {
  [K in keyof (NodeComplexity & NodeSizeMetrics)]-?: number | null
}
export type RelationshipRecord = DirectEdge & { ordinal: number }
export type DiagnosticRecord = {
  severity: string
  path: string
  language: string
  message: string
}
