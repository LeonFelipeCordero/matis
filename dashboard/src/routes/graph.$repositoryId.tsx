import { useQuery } from '@tanstack/react-query'
import { Link, createFileRoute } from '@tanstack/react-router'
import { ArrowLeft, Network, RefreshCw } from 'lucide-react'
import { api, formatDate, repositoriesQuery } from '@/lib/api'
import { Button } from '@/components/ui/button'
import { GraphExplorer } from '@/components/graph/graph-explorer'
import type { ImportedGraph } from '@/lib/api'

export const Route = createFileRoute('/graph/$repositoryId')({
  component: GraphPage,
})

function GraphPage() {
  const { repositoryId } = Route.useParams()
  const repositories = useQuery(repositoriesQuery)
  const graph = useQuery({
    queryKey: ['graph', repositoryId],
    queryFn: () =>
      api<ImportedGraph | null>(
        `/api/architecture?repositoryId=${encodeURIComponent(repositoryId)}`,
      ),
    retry: false,
  })
  const repository = repositories.data?.find(
    (entry) => entry.id === repositoryId,
  )
  const refresh = () => {
    void graph.refetch()
    void repositories.refetch()
  }
  return (
    <main id="main" className="page graph-page">
      <Link to="/" className="back-link">
        <ArrowLeft size={15} />
        All repositories
      </Link>
      <div className="graph-page-heading">
        <div>
          <h1>{repository?.name ?? graph.data?.rootName ?? 'Repository'}</h1>
          <p className="repository-location">{repository?.location}</p>
        </div>
        <div className="graph-actions">
          <Button
            variant="outline"
            onClick={refresh}
            disabled={graph.isFetching}
          >
            <RefreshCw className={graph.isFetching ? 'refreshing' : ''} />
            {graph.isFetching ? 'Refreshing…' : 'Refresh graph'}
          </Button>
        </div>
      </div>
      {graph.isPending && (
        <div className="loading-state graph-loading" role="status">
          Loading the repository graph…
        </div>
      )}
      {graph.error && (
        <div className="error-message" role="alert">
          <h2>Couldn’t load this graph</h2>
          <p>{graph.error.message}</p>
          <Button variant="outline" onClick={refresh}>
            Try again
          </Button>
        </div>
      )}
      {graph.data === null && !graph.error && repository?.name && (
        <WaitingForImport />
      )}
      {graph.data?.state === 'needsReindex' && (
        <section className="waiting-for-import">
          <h2>This graph needs to be re-indexed.</h2>
          <p>The saved index predates normalized graph storage.</p>
        </section>
      )}
      {graph.data?.state === 'ready' && (
        <>
          <div className="import-status">
            <span className="status-dot" />
            Indexed {formatDate(graph.data.importedAt)}
          </div>
          <GraphExplorer
            key={`${repositoryId}:${graph.data.graphRevision}`}
            initial={graph.data}
          />
        </>
      )}
    </main>
  )
}

function WaitingForImport() {
  return (
    <section className="waiting-for-import">
      <div>
        <span className="waiting-icon">
          <Network size={32} />
        </span>
        <h2>Your repository is registered.</h2>
        <p>No graph has been indexed yet.</p>
      </div>
    </section>
  )
}
