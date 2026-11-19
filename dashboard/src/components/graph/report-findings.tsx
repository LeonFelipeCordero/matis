import { AlertTriangle, CircleAlert, CircleCheck, Info } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { GraphView, ViewCycle } from '@/lib/graph-data'

export function ReportFindings({
  view,
  onCycleScope,
}: {
  view: GraphView
  onCycleScope: (cycle: ViewCycle) => void
}) {
  return (
    <section className="report-findings" aria-label="Recorded findings">
      <h3>Recorded findings</h3>
      <p className="report-finding-count">
        {view.cycles.length} import cycle{' '}
        {view.cycles.length === 1 ? 'group' : 'groups'} touching this scope ·{' '}
        {view.diagnostics.length} repository{' '}
        {view.diagnostics.length === 1 ? 'diagnostic' : 'diagnostics'}
      </p>
      <div
        className={`report-cycle-status ${view.cycles.length ? 'report-warning' : 'report-neutral'}`}
      >
        {view.cycles.length ? (
          <AlertTriangle size={18} aria-hidden="true" />
        ) : (
          <CircleCheck size={18} aria-hidden="true" />
        )}
        <div>
          <strong>
            {view.cycles.length
              ? 'Circular imports need review'
              : 'No import cycles recorded'}
          </strong>
          <p>
            {view.cycles.length
              ? 'These files depend on each other. Inspect the groups below.'
              : 'No file-import cycles recorded for this scope. Other issues may still exist.'}
          </p>
        </div>
      </div>
      {view.cycles.map((cycle) => (
        <CycleFinding
          key={cycle.group.id}
          cycle={cycle}
          onScope={onCycleScope}
        />
      ))}
      <details className="report-source" open={view.diagnostics.length > 0}>
        <summary>Repository diagnostics · {view.diagnostics.length}</summary>
        <p className="sidebar-hint">
          Diagnostics cover the entire repository, regardless of the current
          scope.
        </p>
        {view.diagnostics.length ? (
          <ul className="report-connections">
            {view.diagnostics.map((diagnostic, index) => (
              <li
                key={index}
                className={`report-diagnostic report-${diagnostic.severity === 'error' ? 'error' : diagnostic.severity === 'warning' ? 'warning' : 'neutral'}`}
              >
                <div className="report-diagnostic-label">
                  {diagnostic.severity === 'error' ? (
                    <CircleAlert size={16} aria-hidden="true" />
                  ) : diagnostic.severity === 'warning' ? (
                    <AlertTriangle size={16} aria-hidden="true" />
                  ) : (
                    <Info size={16} aria-hidden="true" />
                  )}
                  <strong>{diagnostic.severity}</strong>
                </div>
                <p>{diagnostic.message}</p>
                <span>
                  {diagnostic.path} · {diagnostic.language}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <p>
            No diagnostics recorded. Missing measurements are still shown as
            unavailable.
          </p>
        )}
      </details>
    </section>
  )
}

function CycleFinding({
  cycle,
  onScope,
}: {
  cycle: ViewCycle
  onScope: (cycle: ViewCycle) => void
}) {
  const label = (key: string) => cycle.labels[key] ?? key
  return (
    <details className="report-source report-cycle-finding">
      <summary>
        {cycle.group.nodeStableKeys.length} files in import cycle ·{' '}
        {label(cycle.group.nodeStableKeys[0])}
      </summary>
      <p>
        These files form a mutually reachable import group. The example route is
        not necessarily the whole group. Type-only imports are excluded.
      </p>
      <p className="report-cycle-route">
        {cycle.group.exampleCycle.map(label).join(' → ')}
      </p>
      <p>
        {cycle.outsideCount} {cycle.outsideCount === 1 ? 'file' : 'files'}{' '}
        outside this scope
      </p>
      <ul>
        {cycle.group.nodeStableKeys.map((key) => (
          <li key={key}>{label(key)}</li>
        ))}
      </ul>
      <details className="report-source">
        <summary>Import evidence · {cycle.group.edges.length}</summary>
        <ul className="report-connections">
          {cycle.group.edges.map((edge, index) => (
            <li key={index}>
              <strong>
                {label(edge.sourceStableKey)} → {label(edge.targetStableKey)}
              </strong>
              <span>
                {edge.confidence} · {edge.path || 'Location not recorded'}
                {edge.line != null && `:${edge.line}`}
              </span>
              {edge.evidence && <code>{edge.evidence}</code>}
            </li>
          ))}
        </ul>
      </details>
      <Button variant="outline" size="sm" onClick={() => onScope(cycle)}>
        Report on full group
      </Button>
    </details>
  )
}
