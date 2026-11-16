import { internalNodes } from '../graph-analysis'
import { importCycles } from '../import-cycles.servers'
import { buildProjectGraph, getGraphTrail } from '../project-graph'
import { cycleContainsNode, projectCycles } from '../graph-cycles'
import { hideTestFiles } from '../graph-visibility'
import { graphDetails } from './graph-details'
import type { Architecture, GraphStructure } from '../graph-data'
import type { GraphScope } from '../project-graph-types'
import type { RepositoryService } from './repository-service'
import type { GraphRevisionService } from './graph-revision-service'
import type { NodeService } from './node-service'
import type { RelationshipService } from './relationship-service'
import type { NodeMetricsService } from './node-metrics-service'
import type { DiagnosticService } from './diagnostic-service'

type GraphViewServices = {
  repositories: RepositoryService
  revisions: GraphRevisionService
  nodes: NodeService
  relationships: RelationshipService
  metrics: NodeMetricsService
  diagnostics: DiagnosticService
}

export function graphView(
  id: string,
  scope: GraphScope,
  services: GraphViewServices,
): Architecture | null | undefined {
  const repository = services.repositories.find(id)
  if (!repository) return undefined
  const revision = services.revisions.find(id)
  if (!revision) return null
  const repositoryId = repository.id
  const { importedAt, graphRevision } = revision
  const rootName = revision.rootName ?? repository.name
  if (graphRevision === null)
    return { state: 'needsReindex', repositoryId, importedAt, rootName }
  const nodes = services.nodes.topology(id)
  const structure: GraphStructure = {
    nodes,
    edges: services.relationships.outgoing(id, scope, nodes),
  }
  const visible = hideTestFiles(structure)
  const graph = buildProjectGraph(visible, scope)
  const internal = new Set(internalNodes(nodes).map((node) => node.stableKey))
  const relations = visible.edges.filter(
    (edge) =>
      internal.has(edge.sourceStableKey) && internal.has(edge.targetStableKey),
  )
  const trail = getGraphTrail(
    { nodes: visible.nodes, repository: { name: rootName } },
    scope,
  )
  const groups = importCycles(nodes, services.relationships.imports(id))
  const cycles = (
    projectCycles(
      { ...structure, circularDependencies: { version: 1, groups } },
      graph,
      scope,
    ) ?? []
  ).map((cycle) => ({
    ...cycle,
    labels: Object.fromEntries(
      nodes
        .filter((node) => cycle.group.nodeStableKeys.includes(node.stableKey))
        .map((node) => [node.stableKey, node.path || node.name]),
    ),
    linkedNodeIds: graph.nodes
      .filter((node) => cycleContainsNode(structure, cycle.group, node))
      .map((node) => node.id),
  }))
  const keys = [
    ...new Set([
      ...graph.nodes.map((node) => node.id),
      ...(scope.kind === 'repository' ? [] : [scope.key]),
    ]),
  ]
  const details = graphDetails(id, keys, nodes, services)
  for (const node of graph.nodes) {
    const original = details.nodes.find((entry) => entry.stableKey === node.id)
    if (node.kind === 'function') node.properties = original?.properties
  }
  return {
    state: 'ready',
    repositoryId,
    importedAt,
    graphRevision,
    rootName,
    scope,
    graph,
    relations,
    trail,
    details,
    cycles,
    diagnostics: services.diagnostics.read(id),
  }
}
