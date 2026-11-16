import { integer, primaryKey, sqliteTable, text } from 'drizzle-orm/sqlite-core'

// Maps the shared SQLite schema; Rust remains its sole migration owner.
export const repositories = sqliteTable('repositories', {
  id: text().primaryKey(),
  name: text().notNull(),
  location: text().notNull(),
  createdAt: integer('created_at').notNull(),
})
export const repositoryGraphs = sqliteTable('repository_graphs', {
  repositoryId: text('repository_id').primaryKey(),
  importedAt: integer('imported_at').notNull(),
  graphRevision: text('graph_revision'),
  schemaVersion: integer('schema_version'),
  indexedAt: text('indexed_at'),
  rootName: text('root_name'),
  rootPath: text('root_path'),
})
const nodeIdentity = () => ({
  repositoryId: text('repository_id').notNull(),
  stableKey: text('node_key').notNull(),
})
export const graphNodes = sqliteTable(
  'graph_nodes',
  {
    ...nodeIdentity(),
    parentStableKey: text('parent_key'),
    ordinal: integer().notNull(),
    type: text().notNull(),
    kind: text().notNull(),
    level: text().notNull(),
    name: text().notNull(),
    language: text(),
    qualifiedName: text('qualified_name'),
    path: text(),
  },
  (table) => [primaryKey({ columns: [table.repositoryId, table.stableKey] })],
)
export const graphFiles = sqliteTable(
  'graph_files',
  {
    ...nodeIdentity(),
    bytes: integer('byte_count'),
    sha256: text(),
  },
  (table) => [primaryKey({ columns: [table.repositoryId, table.stableKey] })],
)
export const graphSymbols = sqliteTable(
  'graph_symbols',
  {
    ...nodeIdentity(),
    startLine: integer('start_line'),
    startColumn: integer('start_column'),
    endLine: integer('end_line'),
    endColumn: integer('end_column'),
    signature: text(),
    visibility: text(),
  },
  (table) => [primaryKey({ columns: [table.repositoryId, table.stableKey] })],
)
export const graphMetrics = sqliteTable(
  'graph_node_metrics',
  {
    ...nodeIdentity(),
    lineCount: integer('line_count'),
    parameterCount: integer('parameter_count'),
    baseCyclomaticComplexity: integer('base_complexity'),
    nestedCyclomaticComplexity: integer('nested_complexity'),
    loopNestedComplexity: integer('loop_complexity'),
    cyclomaticComplexity: integer('total_complexity'),
    maxDecisionDepth: integer('max_decision_depth'),
  },
  (table) => [primaryKey({ columns: [table.repositoryId, table.stableKey] })],
)
export const graphDecorators = sqliteTable(
  'graph_node_decorators',
  {
    ...nodeIdentity(),
    ordinal: integer().notNull(),
    decorator: text().notNull(),
  },
  (table) => [
    primaryKey({
      columns: [table.repositoryId, table.stableKey, table.ordinal],
    }),
  ],
)
export const graphEdges = sqliteTable(
  'graph_edges',
  {
    repositoryId: text('repository_id').notNull(),
    ordinal: integer().notNull(),
    sourceStableKey: text('source_key').notNull(),
    targetStableKey: text('target_key').notNull(),
    type: text('relationship').notNull(),
    confidence: text().$type<'EXTRACTED' | 'INFERRED'>().notNull(),
    path: text(),
    line: integer(),
    evidence: text(),
    typeOnly: integer('type_only', { mode: 'boolean' }).notNull(),
  },
  (table) => [primaryKey({ columns: [table.repositoryId, table.ordinal] })],
)
export const graphDiagnostics = sqliteTable(
  'graph_diagnostics',
  {
    repositoryId: text('repository_id').notNull(),
    ordinal: integer().notNull(),
    severity: text().notNull(),
    path: text().notNull(),
    language: text().notNull(),
    message: text().notNull(),
  },
  (table) => [primaryKey({ columns: [table.repositoryId, table.ordinal] })],
)
