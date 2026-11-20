import { expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { fixtureNodes } from '@/lib/project-graph-fixture'
import { GraphReport } from './graph-report'
import { reportView } from './graph-report-test.helpers'
import { GraphSizeMetrics } from './graph-size-metrics'
import type { Artifact, NodeSizeMetrics } from '@/lib/architecture-artifact'

function fixture(key: string, properties?: NodeSizeMetrics): Artifact {
  const nodes = fixtureNodes()
  nodes.find((node) => node.stableKey === key)!.properties = properties
  return { schemaVersion: 2, repository: { name: 'Example' }, nodes, edges: [] }
}

function report(key: string, properties?: NodeSizeMetrics) {
  const artifact = fixture(key, properties)
  const scope =
    key === 'run'
      ? { kind: 'file' as const, key: 'service' }
      : { kind: 'folder' as const, key: 'acme' }
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

test('selected callables display line and parameter counts, including zero parameters', () => {
  const html = report('run', { lineCount: 12, parameterCount: 0 })
  expect(html).toContain('aria-label="Size metrics"')
  expect(html).toContain('<dt>Lines</dt><dd>12</dd>')
  expect(html).toContain('<dt>Parameters</dt><dd>0</dd>')
  expect(html).toContain('including comments and blanks')
})

test('selected files and file metric components display lines only', () => {
  const node = fixture('service', { lineCount: 42 }).nodes.find(
    (entry) => entry.stableKey === 'service',
  )!
  for (const html of [
    report('service', { lineCount: 42 }),
    renderToStaticMarkup(<GraphSizeMetrics node={node} />),
  ]) {
    expect(html).toContain('<dt>Lines</dt><dd>42</dd>')
    expect(html).not.toContain('<dt>Parameters</dt>')
  }
})

test('missing and partial metrics are not presented as zero', () => {
  expect(report('run')).toContain(
    'Re-index this repository to include size metrics',
  )
  expect(report('run', { lineCount: 1 })).toContain(
    '<dt>Parameters</dt><dd>Not available</dd>',
  )
})

test('folders do not display size metrics', () => {
  const artifact = fixture('app', { lineCount: 42 })
  const node = artifact.nodes.find((item) => item.stableKey === 'app')!
  expect(renderToStaticMarkup(<GraphSizeMetrics node={node} />)).toBe('')
})
