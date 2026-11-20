import { Suspense, lazy, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Network } from 'lucide-react'
import { api } from '@/lib/api'
import { Button } from '@/components/ui/button'
import { GraphSidebar } from './graph-sidebar'
import { GraphNavigation } from './graph-navigation'
import { GraphModeNavigation } from './graph-mode-navigation'
import { GraphReport } from './graph-report'
import type { ComponentProps } from 'react'
import type { Architecture, GraphView, ViewCycle } from '@/lib/graph-data'
import type { GraphNode, GraphScope } from '@/lib/project-graph'

const GraphCanvas = lazy(() => import('./graph-canvas'))

function EmptyGraph({ isFile, back }: { isFile: boolean; back?: () => void }) {
  return (
    <div className="empty-graph">
      <Network size={32} />
      <h3>
        {isFile
          ? 'No functions in this file'
          : 'Nothing indexed in this folder'}
      </h3>
      <p>
        {isFile
          ? 'Configuration, styles, and documentation may not contain callable symbols.'
          : 'Only items recorded in the saved index appear here.'}
      </p>
      {back && (
        <Button variant="outline" onClick={back}>
          Back to parent
        </Button>
      )}
    </div>
  )
}

function GraphViewport({
  isFile,
  back,
  ...props
}: ComponentProps<typeof GraphCanvas> & {
  isFile: boolean
  back?: () => void
}) {
  if (!props.graph.nodes.length)
    return <EmptyGraph isFile={isFile} back={back} />
  return (
    <Suspense
      fallback={
        <div className="loading-state" role="status">
          Drawing your graph…
        </div>
      }
    >
      <GraphCanvas {...props} />
    </Suspense>
  )
}

function GraphWorkspace({
  view,
  navigate,
  mode,
  onMode,
  ...actions
}: {
  view: GraphView
  mode: 'explore' | 'report'
  onMode: (mode: 'explore' | 'report') => void
  navigate: (index: number) => void
  selected: string | null
  focusId: string | null
  onSelect: (id: string | null) => void
  onOpen: (node: GraphNode) => void
  onCycleScope: (cycle: ViewCycle) => void
}) {
  const { trail, graph, details } = view
  const current = trail[trail.length - 1]
  return (
    <section className="explorer" aria-label="Project graph">
      <GraphModeNavigation mode={mode} onMode={onMode} />
      <GraphNavigation trail={trail} navigate={navigate} />
      {mode === 'report' ? (
        <GraphReport
          key={JSON.stringify(current.scope)}
          view={view}
          {...actions}
          onExplore={(node) => {
            if (node) actions.onOpen(node)
            onMode('explore')
          }}
        />
      ) : (
        <div className="graph-workspace">
          <div
            className="graph-canvas"
            role="region"
            aria-label={`${current.label} dependency graph`}
          >
            <GraphViewport
              key={JSON.stringify(current.scope)}
              graph={graph}
              selected={actions.selected}
              focusId={actions.focusId}
              onSelect={actions.onSelect}
              onOpen={actions.onOpen}
              isFile={current.scope.kind === 'file'}
              back={
                trail.length > 1 ? () => navigate(trail.length - 2) : undefined
              }
            />
          </div>
          <GraphSidebar
            key={JSON.stringify(current.scope)}
            graph={graph}
            details={details}
            selected={actions.selected}
            onSelect={actions.onSelect}
            onOpen={actions.onOpen}
          />
        </div>
      )}
    </section>
  )
}

function useGraphView(initial: GraphView, scope: GraphScope) {
  return useQuery({
    queryKey: [
      'graph-view',
      initial.repositoryId,
      initial.graphRevision,
      scope,
    ],
    queryFn: () => {
      const parameters = new URLSearchParams({
        repositoryId: initial.repositoryId,
        kind: scope.kind,
      })
      if (scope.kind !== 'repository') parameters.set('key', scope.key)
      return api<Architecture | null>(`/api/architecture?${parameters}`)
    },
    initialData: scope.kind === 'repository' ? initial : undefined,
    retry: false,
  })
}

export function GraphExplorer({ initial }: { initial: GraphView }) {
  const [scope, setScope] = useState<GraphScope>({ kind: 'repository' })
  const [selected, setSelected] = useState<string | null>(null)
  const [mode, setMode] = useState<'explore' | 'report'>('explore')
  const view = useGraphView(initial, scope)
  if (
    view.error ||
    (view.data && view.data.state !== 'ready') ||
    view.data === null
  )
    return (
      <div className="error-message" role="alert">
        <p>{view.error?.message ?? 'This graph needs to be re-indexed.'}</p>
        <Button
          variant="outline"
          onClick={() => {
            void view.refetch()
          }}
        >
          Try again
        </Button>
        <Button
          variant="outline"
          onClick={() => setScope({ kind: 'repository' })}
        >
          Back to repository
        </Button>
      </div>
    )
  if (!view.data)
    return (
      <div className="loading-state" role="status">
        Loading the repository graph…
      </div>
    )
  const data = view.data
  const open = (node: GraphNode) => {
    const destination = node.destination
    const nextScope = destination?.scope ?? node.scope
    if (!nextScope) return
    setScope(nextScope)
    setSelected(destination?.focusId ?? null)
  }
  const navigate = (index: number) => {
    setScope(data.trail[index].scope)
    setSelected(null)
  }
  return (
    <GraphWorkspace
      view={data}
      navigate={navigate}
      mode={mode}
      onMode={setMode}
      selected={selected}
      focusId={selected}
      onSelect={setSelected}
      onOpen={open}
      onCycleScope={(cycle) => {
        setScope(cycle.scope)
        setSelected(null)
      }}
    />
  )
}
