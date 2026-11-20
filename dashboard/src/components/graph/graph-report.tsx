import { ArrowUpRight, Info } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ReportFindings } from './report-findings'
import { ReportItems } from './report-items'
import type { ReportItemProps } from './report-items'
import type { ViewCycle } from '@/lib/graph-data'

export function GraphReport({
  onCycleScope,
  ...props
}: ReportItemProps & {
  onCycleScope: (cycle: ViewCycle) => void
}) {
  const { view, onExplore } = props
  const path = view.trail.map((part) => part.label).join(' / ')
  return (
    <div className="report-workspace">
      <header className="report-intro">
        <div>
          <h2>Repository report</h2>
          <p>
            {path}{' '}
            <span className="report-snapshot">
              Saved index · {new Date(view.importedAt).toLocaleDateString()}
            </span>
          </p>
        </div>
        <Button variant="outline" onClick={() => onExplore()}>
          View scope in graph <ArrowUpRight size={15} aria-hidden="true" />
        </Button>
      </header>
      <div className="report-body">
        <aside className="report-overview" aria-label="Report overview">
          <ReportFindings view={view} onCycleScope={onCycleScope} />
          <div className="report-reading-note">
            <Info size={16} aria-hidden="true" />
            <p>
              Measurements are not verdicts. Check limits and pass/fail results
              are not available in this report.
            </p>
          </div>
          <details className="report-source report-coverage">
            <summary>Scope and measurement coverage</summary>
            <p>
              This report uses the saved index imported{' '}
              {new Date(view.importedAt).toLocaleString()}. It does not inspect
              current source files.
            </p>
            <p>
              {view.details.nodes.length} item records available at this level.
              Open a folder or file report to inspect its children. Conventional
              test files are hidden from navigation. Outside-repository
              references are excluded; outgoing internal references outside this
              scope appear separately.
            </p>
            <p>
              Dependency metrics use the full index, including tests and
              type-only dependencies. File complexity sums include owned
              functions and methods. Missing values are not zero. Thresholds are
              unavailable here.
            </p>
          </details>
        </aside>
        <ReportItems {...props} />
      </div>
    </div>
  )
}
