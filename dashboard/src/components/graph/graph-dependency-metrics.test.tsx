import { expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { fixtureNodes } from '@/lib/project-graph-fixture'
import { GraphReport } from './graph-report'
import { GraphDependencyMetrics } from './graph-dependency-metrics'
import { reportView } from './graph-report-test.helpers'
import type {
  Artifact,
  NodeDependencyMetrics,
} from '@/lib/architecture-artifact'

function fixture(): Artifact {
  return {
    schemaVersion: 2,
    repository: { name: 'Example' },
    nodes: fixtureNodes(),
    edges: [],
  }
}

function report(key: string, properties?: NodeDependencyMetrics) {
  const artifact = fixture()
  const node = artifact.nodes.find((entry) => entry.stableKey === key)!
  node.properties = properties
  const scope =
    key === 'run'
      ? { kind: 'file' as const, key: 'service' }
      : key === 'service'
        ? { kind: 'folder' as const, key: 'acme' }
        : { kind: 'repository' as const }
  return renderToStaticMarkup(
    <GraphReport
      view={reportView(artifact, scope)}
      selected={key}
      onSelect={() => {}}
      onOpen={() => {}}
      onExplore={() => {}}
      onCycleScope={() => {}}
    />,
  )
}

const metrics = {
  fanIn: 2,
  fanOut: 3,
  instability: 0.6,
  transitiveChangeImpact: 5,
}

test('report shows indexed dependency metrics for folders, files, and callables', () => {
  for (const key of ['app', 'service', 'run']) {
    const html = report(key, metrics)
    expect(html).toContain('aria-label="Dependency metrics"')
    expect(html).toContain('>Fan-in</dt><dd>2</dd>')
    expect(html).toContain('>Fan-out</dt><dd>3</dd>')
    expect(html).toContain('>Internal instability</dt><dd>0.60</dd>')
    expect(html).toContain('>Transitive change impact</dt><dd>5</dd>')
    expect(html).toContain('distinct internal')
    expect(html).toContain('full index, including tests and type-only')
    expect(html).toContain('Self and external dependencies are excluded')
    expect(html).toContain('fan-out / (fan-in + fan-out); N/A means isolated')
  }
})

test('missing metrics require reindexing; indexed zeros and partial values remain distinct', () => {
  const legacy = report('run')
  expect(legacy).toContain(
    'Re-index this repository to include dependency metrics',
  )
  expect(legacy).not.toContain('>Fan-in</dt>')
  expect(legacy).not.toContain('>Internal instability</dt>')
  const partial = report('run', { fanIn: 0 })
  expect(partial).toContain('>Fan-in</dt><dd>0</dd>')
  expect(partial).toContain('>Fan-out</dt><dd>Not indexed</dd>')
  expect(partial).toContain('>Internal instability</dt><dd>Not indexed</dd>')
  expect(partial).toContain(
    '>Transitive change impact</dt><dd>Not indexed</dd>',
  )
})

test('internal instability formats ratios, including zero, separately from isolated and missing values', () => {
  for (const [instability, display] of [
    [0, '0.00'],
    [1, '1.00'],
    [1 / 3, '0.33'],
    [null, 'N/A'],
    [undefined, 'Not indexed'],
  ] as const) {
    const html = report('run', { ...metrics, instability })
    expect(html).toContain(`>Internal instability</dt><dd>${display}</dd>`)
  }
  expect(report('run', { instability: null })).toContain(
    '>Internal instability</dt><dd>N/A</dd>',
  )
})

test('file metric component displays indexed dependency metrics', () => {
  const node = fixture().nodes.find((entry) => entry.stableKey === 'service')!
  node.properties = metrics
  const html = renderToStaticMarkup(<GraphDependencyMetrics node={node} />)
  expect(html).toContain('>Fan-in</dt><dd>2</dd>')
  expect(html).toContain('>Internal instability</dt><dd>0.60</dd>')
})
