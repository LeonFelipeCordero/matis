import { expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { buildProjectGraph } from '@/lib/project-graph'
import { GraphSidebar } from './graph-sidebar'
import { GraphReport } from './graph-report'
import { reportView } from './graph-report-test.helpers'
import type { Artifact, NodeComplexity } from '@/lib/architecture-artifact'

const metrics = {
  baseCyclomaticComplexity: 5,
  nestedCyclomaticComplexity: 2,
  loopNestedComplexity: 1,
  cyclomaticComplexity: 8,
  maxDecisionDepth: 4,
}

function fixture(properties?: NodeComplexity): Artifact {
  const root = {
    stableKey: 'repo',
    type: 'repository',
    kind: 'repository',
    level: 'repository',
    name: 'Example',
    path: '.',
    language: null,
    parentStableKey: null,
    ancestorStableKeys: [],
    depth: 0,
  }
  const file = {
    ...root,
    stableKey: 'file',
    type: 'file',
    kind: 'file',
    level: 'file',
    name: 'main.ts',
    path: 'main.ts',
    parentStableKey: 'repo',
    ancestorStableKeys: ['repo'],
    depth: 1,
  }
  return {
    schemaVersion: 2,
    repository: { name: 'Example' },
    edges: [],
    nodes: [
      root,
      file,
      {
        ...file,
        stableKey: 'run',
        type: 'method',
        kind: 'method',
        level: 'callable',
        name: 'run',
        parentStableKey: 'file',
        ancestorStableKeys: ['repo', 'file'],
        depth: 2,
        properties: properties ? { ...properties } : undefined,
      },
    ],
  }
}

function report(artifact: Artifact, selected = 'run', file = true) {
  return renderToStaticMarkup(
    <GraphReport
      view={reportView(
        artifact,
        file ? { kind: 'file', key: 'file' } : { kind: 'repository' },
      )}
      selected={selected}
      onSelect={() => {}}
      onOpen={() => {}}
      onExplore={() => {}}
      onCycleScope={() => {}}
    />,
  )
}

test('Explore keeps connections without analysis, Report shows measurements and an explore action', () => {
  const artifact = fixture(metrics)
  const view = reportView(artifact, { kind: 'file', key: 'file' })
  const explore = renderToStaticMarkup(
    <GraphSidebar
      graph={view.graph}
      details={view.details}
      selected="run"
      onSelect={() => {}}
      onOpen={() => {}}
    />,
  )
  expect(explore).toContain('Connections')
  expect(explore).not.toContain('Weighted complexity')
  expect(explore).not.toContain('Circular dependencies')
  expect(explore).not.toContain('Dependency metrics')
  const html = report(artifact)
  expect(html).toContain('Weighted complexity')
  expect(html).toContain('Explore run')
  expect(html).not.toContain('<h4>Connections</h4>')
})

test('imported complexity survives graph projection and displays its breakdown', () => {
  const artifact = fixture(metrics)
  const graph = buildProjectGraph(artifact, { kind: 'file', key: 'file' })
  expect(graph.nodes[0].properties).toEqual(metrics)
  const html = report(artifact)
  expect(html).toContain('aria-label="Weighted complexity"')
  expect(html).toContain('<h4>Weighted complexity</h4>')
  expect(html).not.toContain('aria-label="Cyclomatic complexity"')
  expect(html).not.toContain('Base complexity')
  for (const [label, value] of [
    ['Total', 8],
    ['Cyclomatic complexity', 5],
    ['Nested decisions extra', 2],
    ['Nested loops extra', 1],
    ['Maximum decision depth', 4],
  ])
    expect(html).toContain(`<dt>${label}</dt><dd>${value}</dd>`)
})

test('zero penalties display, missing metrics never become zero', () => {
  expect(
    report(
      fixture({
        ...metrics,
        nestedCyclomaticComplexity: 0,
        loopNestedComplexity: 0,
      }),
    ),
  ).toContain('<dt>Nested loops extra</dt><dd>0</dd>')
  const legacy = report(fixture())
  expect(legacy).toContain('Not available. Re-index this repository')
  expect(legacy).toContain('<h4>Weighted complexity</h4>')
  expect(legacy).not.toContain('<dt>Total</dt>')
  const partial = report(fixture({ cyclomaticComplexity: 1 }))
  expect(partial).toContain('<dt>Total</dt><dd>1</dd>')
  expect(partial).not.toContain('Nested loops extra')
})

test('containers and constructors do not show complexity', () => {
  const artifact = fixture(metrics)
  expect(report(artifact, 'file', false)).not.toContain(
    'aria-label="Weighted complexity"',
  )
  artifact.nodes[2].type = artifact.nodes[2].kind = 'constructor'
  expect(report(artifact)).not.toContain('aria-label="Weighted complexity"')
})
