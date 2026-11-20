import { useState } from 'react'
import {
  ArrowLeft,
  ArrowRight,
  Braces,
  ChevronRight,
  FileCode2,
  Folder,
  Search,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import type { ArtifactNode } from '@/lib/architecture-artifact'
import type { GraphDetails } from '@/lib/graph-data'
import type { GraphNode, ProjectGraph } from '@/lib/project-graph'

type Props = {
  graph: ProjectGraph
  details: GraphDetails
  selected: string | null
  onSelect: (id: string | null) => void
  onOpen: (node: GraphNode) => void
}

export function GraphSidebar({
  graph,
  details,
  selected,
  onSelect,
  onOpen,
}: Props) {
  const [search, setSearch] = useState('')
  const item = graph.nodes.find((node) => node.id === selected)
  if (item)
    return (
      <NodeDetails
        item={item}
        graph={graph}
        details={details}
        onSelect={onSelect}
        onOpen={onOpen}
      />
    )
  const matches = graph.nodes.filter((node) =>
    `${node.label} ${node.path}`.toLowerCase().includes(search.toLowerCase()),
  )
  return (
    <aside className="graph-sidebar" aria-label="Graph contents">
      <div className="sidebar-heading">
        <h3>In this view</h3>
        <span>{graph.nodes.length}</span>
      </div>
      <div className="search-field">
        <Search size={16} />
        <Input
          aria-label="Find in this view"
          placeholder="Find a folder, file, or function"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
      </div>
      <div className="scope-items">
        {matches.map((node) => (
          <ScopeItem
            key={node.id}
            node={node}
            onSelect={onSelect}
            onOpen={onOpen}
          />
        ))}
        {!matches.length && (
          <p className="sidebar-hint">
            {search
              ? 'No matching items. Try another name or path.'
              : 'No items at this level.'}
          </p>
        )}
      </div>
    </aside>
  )
}

function ScopeItem({
  node,
  onSelect,
  onOpen,
}: {
  node: GraphNode
  onSelect: Props['onSelect']
  onOpen: Props['onOpen']
}) {
  const Icon =
    node.kind === 'folder' ? Folder : node.kind === 'file' ? FileCode2 : Braces
  return (
    <div className="scope-item">
      <button
        className="scope-select"
        onClick={() => onSelect(node.id)}
        title={node.path}
      >
        <Icon size={16} />
        <span>
          <strong>{node.label}</strong>
          <small>
            {node.role === 'boundary' ? 'Outside scope · ' : ''}
            {node.path || node.detail}
          </small>
        </span>
      </button>
      {(node.scope || node.destination) && (
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={() => onOpen(node)}
          aria-label={`Open ${node.path || node.label}`}
        >
          <ChevronRight />
        </Button>
      )}
    </div>
  )
}

function nodeTitle(item: GraphNode, original: ArtifactNode | undefined) {
  if (item.kind !== 'function' || !item.path) return item.path || item.label
  return `${item.path}${original?.startLine ? `:${original.startLine}` : ''} · ${item.label}`
}

function NodeSummary({
  item,
  original,
  onOpen,
}: {
  item: GraphNode
  original: ArtifactNode | undefined
  onOpen: Props['onOpen']
}) {
  return (
    <>
      <h3>{nodeTitle(item, original)}</h3>
      <p className="eyebrow">
        {item.kind}
        {item.role === 'boundary' ? ' · Outside scope' : ''}
      </p>
      {(item.scope || item.destination) && (
        <Button className="open-selected" onClick={() => onOpen(item)}>
          {item.destination ? 'Go to function' : `Explore ${item.kind}`}
          <ArrowRight />
        </Button>
      )}
    </>
  )
}

function NodeDetails({
  item,
  graph,
  details,
  onSelect,
  onOpen,
}: Omit<Props, 'selected'> & { item: GraphNode }) {
  const original = details.nodes.find((node) => node.stableKey === item.id)
  const connections = graph.edges.filter(
    (edge) => edge.source === item.id || edge.target === item.id,
  )
  const lookup = new Map(graph.nodes.map((node) => [node.id, node]))
  return (
    <aside className="graph-sidebar node-details" aria-label="Selected item">
      <Button
        variant="ghost"
        size="sm"
        className="back-to-list"
        onClick={() => onSelect(null)}
      >
        <ArrowLeft />
        All items
      </Button>
      <NodeSummary item={item} original={original} onOpen={onOpen} />
      <div className="section-heading">
        <h4>Connections</h4>
        <span>{connections.length}</span>
      </div>
      <div className="connection-list">
        {connections.map((edge) => {
          const outgoing = edge.source === item.id
          const other = lookup.get(outgoing ? edge.target : edge.source)!
          return (
            <button
              key={edge.id}
              className="connection"
              onClick={() => onSelect(other.id)}
              title={other.path}
            >
              <span>
                {outgoing ? <ArrowRight size={15} /> : <ArrowLeft size={15} />}
                {outgoing ? 'Uses' : 'Used by'} <strong>{other.label}</strong>
              </span>
            </button>
          )
        })}
        {!connections.length && (
          <p className="sidebar-hint">
            No connections recorded here. This doesn’t imply the item is unused.
          </p>
        )}
      </div>
    </aside>
  )
}
