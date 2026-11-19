import { fixtureDetails } from '@/lib/graph-details-test.helpers'
import { buildProjectGraph } from '@/lib/project-graph'
import type { Artifact } from '@/lib/architecture-artifact'
import type { GraphView } from '@/lib/graph-data'
import type { GraphScope } from '@/lib/project-graph'

export function reportView(
  artifact: Artifact,
  scope: GraphScope = { kind: 'repository' },
): GraphView {
  return {
    state: 'ready',
    repositoryId: 'repo',
    importedAt: 0,
    rootName: artifact.repository.name,
    graphRevision: 'revision',
    scope,
    trail: [{ label: artifact.repository.name, scope }],
    graph: buildProjectGraph(artifact, scope),
    details: fixtureDetails(artifact),
    relations: artifact.edges,
    cycles: [],
    diagnostics: [],
  }
}
