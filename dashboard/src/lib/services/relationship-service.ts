import type { RelationshipRepositoryPort } from '../repository-ports'
import type { ArtifactNode } from '../architecture-artifact'
import type { GraphScope } from '../project-graph-types'

export class RelationshipService {
  constructor(private readonly relationships: RelationshipRepositoryPort) {}

  outgoing(id: string, scope: GraphScope, nodes: Array<ArtifactNode>) {
    const keys =
      scope.kind === 'repository'
        ? undefined
        : nodes
            .filter(
              (node) =>
                node.stableKey === scope.key ||
                node.ancestorStableKeys.includes(scope.key),
            )
            .map((node) => node.stableKey)
    return this.relationships
      .outgoing(id, keys)
      .map(({ ordinal: _ordinal, ...edge }) => edge)
  }

  relations(id: string) {
    return this.relationships.relations(id)
  }

  imports(id: string) {
    return this.relationships.imports(id)
  }
}
