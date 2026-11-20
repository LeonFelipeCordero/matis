import {
  ArrowUpRight,
  Braces,
  ChevronRight,
  FileCode2,
  Folder,
} from 'lucide-react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ReportItem } from './report-item'
import type { ArtifactNode } from '@/lib/architecture-artifact'
import type { GraphView } from '@/lib/graph-data'
import type { GraphNode } from '@/lib/project-graph'

export type ReportItemProps = {
  view: GraphView
  selected: string | null
  onSelect: (id: string | null) => void
  onOpen: (node: GraphNode) => void
  onExplore: (node?: GraphNode) => void
}

export function ReportItems(props: ReportItemProps) {
  const [search, setSearch] = useState('')
  const [kind, setKind] = useState('all')
  const { view } = props
  const lookup = new Map(view.graph.nodes.map((node) => [node.id, node]))
  const matches = view.details.nodes.filter(
    (node) =>
      `${node.name} ${node.path} ${node.qualifiedName ?? ''}`
        .toLowerCase()
        .includes(search.toLowerCase()) &&
      (kind === 'all' ||
        (kind === 'boundary'
          ? lookup.get(node.stableKey)?.role === 'boundary'
          : node.level === kind)),
  )
  const local = matches.filter(
    (node) => lookup.get(node.stableKey)?.role !== 'boundary',
  )
  const boundary = matches.filter(
    (node) => lookup.get(node.stableKey)?.role === 'boundary',
  )
  return (
    <section aria-label="Item measurements" className="report-measurements">
      <div className="report-section-heading">
        <h3>Item measurements</h3>
        <span>
          {view.details.nodes.length}{' '}
          {view.details.nodes.length === 1 ? 'item' : 'items'} indexed
        </span>
      </div>
      <p className="sidebar-hint">
        Select an item to inspect measurements and evidence. Open its report to
        go one level deeper.
      </p>
      <div className="report-controls">
        <label>
          Find an item
          <Input
            placeholder="Name, path, or qualified name"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>
        <label>
          Item type
          <select
            value={kind}
            onChange={(event) => setKind(event.target.value)}
          >
            <option value="all">All items</option>
            <option value="module">Folders</option>
            <option value="file">Files</option>
            <option value="callable">Functions and methods</option>
            <option value="boundary">Outside scope</option>
          </select>
        </label>
      </div>
      <p className="report-result-count" role="status">
        {matches.length} of {view.details.nodes.length} items
      </p>
      {!matches.length && (
        <p>No matching items. Change the search or item type.</p>
      )}
      <ItemGroup title="In this scope" nodes={local} {...props} />
      <ItemGroup
        title="Outside scope · outgoing references"
        nodes={boundary}
        {...props}
      />
    </section>
  )
}

function ItemGroup({
  title,
  nodes,
  ...props
}: ReportItemProps & { title: string; nodes: Array<ArtifactNode> }) {
  if (!nodes.length) return null
  return (
    <section className="report-item-group">
      <h4>
        {title} · {nodes.length}
      </h4>
      {nodes.map((node) => (
        <ItemRecord key={node.stableKey} node={node} {...props} />
      ))}
    </section>
  )
}

function ItemRecord({
  node,
  view,
  selected,
  onSelect,
  onOpen,
  onExplore,
}: ReportItemProps & { node: ArtifactNode }) {
  const item = view.graph.nodes.find((entry) => entry.id === node.stableKey)
  const expanded = selected === node.stableKey
  const Icon =
    node.level === 'module'
      ? Folder
      : node.level === 'file'
        ? FileCode2
        : Braces
  return (
    <div className={`report-record${expanded ? ' is-expanded' : ''}`}>
      <details open={expanded}>
        <summary
          onClick={(event) => {
            event.preventDefault()
            onSelect(expanded ? null : node.stableKey)
          }}
        >
          <ChevronRight
            className="report-row-chevron"
            size={16}
            aria-hidden="true"
          />
          <span className="report-item-icon" aria-hidden="true">
            <Icon size={19} />
          </span>
          <span className="report-row-name">
            <strong>{node.name}</strong>
            <span>{node.path || node.qualifiedName || node.kind}</span>
          </span>
          <span className="report-row-measurements">
            {node.properties?.cyclomaticComplexity != null && (
              <span>
                <strong>{String(node.properties.cyclomaticComplexity)}</strong>{' '}
                Weighted complexity
              </span>
            )}
            {node.properties?.lineCount != null && (
              <span>
                <strong>{String(node.properties.lineCount)}</strong> lines
              </span>
            )}
            <small>{node.kind}</small>
          </span>
        </summary>
        {expanded && (
          <>
            <ReportItem
              node={node}
              item={item}
              view={view}
              onExplore={onExplore}
            />
            {item?.scope && (
              <Button
                className="report-drill"
                variant="outline"
                size="sm"
                onClick={() => onOpen(item)}
              >
                Report on {node.name}{' '}
                <ChevronRight size={14} aria-hidden="true" />
              </Button>
            )}
          </>
        )}
      </details>
      {!expanded && item?.scope && (
        <ItemActions
          item={item}
          name={node.name}
          onOpen={onOpen}
          onExplore={onExplore}
        />
      )}
    </div>
  )
}

function ItemActions({
  item,
  name,
  onOpen,
  onExplore,
}: Pick<ReportItemProps, 'onOpen' | 'onExplore'> & {
  item: GraphNode
  name: string
}) {
  return (
    <div className="report-row-actions">
      <Button
        variant="ghost"
        size="sm"
        onClick={() => onOpen(item)}
        aria-label={`Open report for ${name}`}
      >
        Open report <ChevronRight size={14} aria-hidden="true" />
      </Button>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => onExplore(item)}
        aria-label={`View in graph: ${name}`}
      >
        View in graph <ArrowUpRight size={14} aria-hidden="true" />
      </Button>
    </div>
  )
}
