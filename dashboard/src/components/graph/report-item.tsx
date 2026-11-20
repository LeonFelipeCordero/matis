import { Button } from '@/components/ui/button'
import { GraphComplexity } from './graph-complexity'
import { GraphDependencyMetrics } from './graph-dependency-metrics'
import { GraphSizeMetrics } from './graph-size-metrics'
import { GraphFileComplexity } from './graph-file-complexity'
import type { ArtifactNode } from '@/lib/architecture-artifact'
import type { GraphView } from '@/lib/graph-data'
import type { GraphNode } from '@/lib/project-graph'

export function ReportItem({
  node,
  view,
  item,
  onExplore,
}: {
  node: ArtifactNode
  view: GraphView
  item?: GraphNode
  onExplore: (item: GraphNode) => void
}) {
  const sum = Object.hasOwn(view.details.fileComplexities, node.stableKey)
    ? view.details.fileComplexities[node.stableKey]
    : undefined
  return (
    <div className="report-item-detail">
      <div className="report-location">
        <p>
          {node.path || node.name}
          {node.startLine != null &&
            `:${node.startLine}${node.endLine != null ? `–${node.endLine}` : ''}`}{' '}
          · {node.language || 'Language not recorded'} · {node.kind}
        </p>
        {item && (
          <Button variant="outline" size="sm" onClick={() => onExplore(item)}>
            Explore {node.name}
          </Button>
        )}
      </div>
      {node.qualifiedName && (
        <p className="report-qualified">{node.qualifiedName}</p>
      )}
      {node.signature && (
        <pre className="report-signature">
          <code>{node.signature}</code>
        </pre>
      )}
      <div className="report-metrics">
        {['function', 'method'].includes(node.type) && (
          <div>
            <GraphComplexity properties={node.properties} />
            <p className="sidebar-hint">
              Weighted total includes base decisions plus extra weight for
              nested decisions and loops. It is not a quality score.
            </p>
          </div>
        )}
        {sum && <GraphFileComplexity sum={sum} />}
        <GraphSizeMetrics node={node} />
        <GraphDependencyMetrics node={node} />
      </div>
      <SourceRecord node={node} />
      <ReportConnections node={node} view={view} />
      <RelationEvidence node={node} view={view} />
    </div>
  )
}

function RelationEvidence({
  node,
  view,
}: {
  node: ArtifactNode
  view: GraphView
}) {
  const edges = view.relations.filter(
    (edge) =>
      edge.sourceStableKey === node.stableKey ||
      edge.targetStableKey === node.stableKey,
  )
  const lookup = new Map(
    view.details.nodes.map((entry) => [entry.stableKey, entry]),
  )
  const label = (key: string) =>
    lookup.get(key)?.qualifiedName ||
    lookup.get(key)?.path ||
    lookup.get(key)?.name ||
    key
  return (
    <details className="report-source">
      <summary>Direct relation evidence · {edges.length}</summary>
      <p className="sidebar-hint">
        Direct records for this item in the scoped response, not all descendants
        or incoming references from outside scope. Unresolved labels retain the
        recorded key.
      </p>
      {!edges.length && (
        <p>
          No direct relation evidence available for this item in this response.
        </p>
      )}
      <ul className="report-connections">
        {edges.map((edge, index) => (
          <li key={index}>
            <strong>
              {label(edge.sourceStableKey)} → {label(edge.targetStableKey)}
            </strong>
            <span>
              {edge.type} · {edge.confidence}
              {edge.typeOnly && ' · Type-only'} ·{' '}
              {edge.path || 'Location not recorded'}
              {edge.line != null && `:${edge.line}`}
            </span>
            {edge.evidence && <code>{edge.evidence}</code>}
          </li>
        ))}
      </ul>
    </details>
  )
}

function SourceRecord({ node }: { node: ArtifactNode }) {
  const properties = node.properties
  const decorators = properties?.decorators
  const rows = [
    ['Visibility', properties?.visibility],
    ['Bytes', properties?.bytes],
    ['SHA-256', properties?.sha256],
    ['Start column', node.startColumn],
    ['End column', node.endColumn],
    [
      'Decorators',
      Array.isArray(decorators) ? decorators.join(', ') : undefined,
    ],
  ].filter(([, value]) => value != null)
  if (!rows.length) return null
  return (
    <details className="report-source">
      <summary>Source record</summary>
      <dl>
        {rows.map(([label, value]) => (
          <div key={String(label)}>
            <dt>{String(label)}</dt>
            <dd>{String(value)}</dd>
          </div>
        ))}
      </dl>
    </details>
  )
}

function ReportConnections({
  node,
  view,
}: {
  node: ArtifactNode
  view: GraphView
}) {
  const connections = view.graph.edges.filter(
    (edge) => edge.source === node.stableKey || edge.target === node.stableKey,
  )
  const lookup = new Map(view.graph.nodes.map((item) => [item.id, item]))
  return (
    <details className="report-source">
      <summary>Connections in this view · {connections.length}</summary>
      <p className="sidebar-hint">
        Grouped connections at this navigation level, not full-index fan-in or
        fan-out. Outside-scope items are outgoing references only.
      </p>
      <ul className="report-connections">
        {connections.map((edge) => {
          const outgoing = edge.source === node.stableKey
          const other = lookup.get(outgoing ? edge.target : edge.source)
          return (
            <li key={edge.id}>
              <strong>
                {outgoing ? 'Uses' : 'Used by'} {other?.path || other?.label}
              </strong>
              {other?.role === 'boundary' && ' · Outside scope'}
              <span>
                {Object.entries(edge.relationships)
                  .map(([kind, count]) => `${kind}: ${count}`)
                  .join(' · ')}{' '}
                · {edge.count} recorded relations
              </span>
            </li>
          )
        })}
      </ul>
      {!connections.length && (
        <p>
          No connections recorded in this view. This does not imply the item is
          unused.
        </p>
      )}
    </details>
  )
}
