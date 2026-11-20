import { expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { fixtureDetails } from '@/lib/graph-details-test.helpers'
import { fixtureNodes } from '@/lib/project-graph-fixture'
import { buildProjectGraph } from '@/lib/project-graph'
import { GraphReport } from './graph-report'
import { GraphSidebar } from './graph-sidebar'
import { ReportFindings } from './report-findings'
import type { GraphView } from '@/lib/graph-data'

function fixture(): GraphView {
  const nodes = fixtureNodes()
  const run = nodes.find((node) => node.stableKey === 'run')!
  Object.assign(run, {
    signature: 'run(value: str)',
    startLine: 12,
    endLine: 18,
    properties: {
      cyclomaticComplexity: 17,
      baseCyclomaticComplexity: 6,
      lineCount: 7,
      parameterCount: 0,
      fanIn: 0,
      visibility: 'public',
    },
  })
  const artifact = {
    schemaVersion: 2 as const,
    repository: { name: 'Example' },
    nodes,
    edges: [],
  }
  return {
    state: 'ready',
    repositoryId: 'repo',
    importedAt: 0,
    rootName: 'Example',
    graphRevision: 'revision',
    scope: { kind: 'file', key: 'service' },
    trail: [{ label: 'service.py', scope: { kind: 'file', key: 'service' } }],
    graph: buildProjectGraph(artifact, { kind: 'file', key: 'service' }),
    details: fixtureDetails(artifact),
    relations: [
      {
        sourceStableKey: 'run',
        targetStableKey: 'stop',
        type: 'CALLS',
        confidence: 'EXTRACTED',
        path: 'service.py',
        line: 14,
        evidence: 'stop()',
      },
    ],
    diagnostics: [
      {
        severity: 'warning',
        path: 'other.py',
        language: 'python',
        message: 'Recorded parse issue',
      },
    ],
    cycles: [
      {
        group: {
          id: 'cycle',
          nodeStableKeys: ['service', 'other'],
          exampleCycle: ['service', 'other', 'service'],
          edges: [
            {
              sourceStableKey: 'service',
              targetStableKey: 'other',
              type: 'IMPORTS',
              confidence: 'EXTRACTED',
              path: 'service.py',
              line: 2,
              evidence: 'import other',
            },
          ],
        },
        labels: { service: 'service.py', other: 'other.py' },
        linkedNodeIds: ['run'],
        outsideCount: 1,
        scope: { kind: 'repository' },
        nodeIds: [],
        edges: [],
      },
    ],
  }
}
const actions = {
  onSelect: () => {},
  onOpen: () => {},
  onExplore: () => {},
  onCycleScope: () => {},
}

test('Report renders actual measurements, source evidence and honest coverage', () => {
  const html = renderToStaticMarkup(
    <GraphReport view={fixture()} selected="run" {...actions} />,
  )
  for (const text of [
    'Recorded findings',
    '1 import cycle group',
    'Recorded parse issue',
    'other.py',
    'run(value: str)',
    'Weighted complexity',
    '>17</dd>',
    '>Parameters</dt><dd>0</dd>',
    '>Fan-in</dt><dd>0</dd>',
    'Not indexed',
    'service.py:14',
    'stop()',
    'public',
    'Thresholds are unavailable',
    'including tests and type-only',
    'Source record',
    'Direct relation evidence',
    'import other',
    'service.py:2',
    '1 file outside this scope',
  ])
    expect(html).toContain(text)
  expect(html).not.toContain('react-flow')
  expect(html).not.toContain('dependency graph')
})

test('report cycle disclosure keeps full-group navigation separate from selection', () => {
  const view = fixture()
  let scoped: GraphView['cycles'][number] | undefined
  const findings = ReportFindings({
    view,
    onCycleScope: (cycle) => {
      scoped = cycle
    },
  })
  const cycles = findings.props.children[3]
  const disclosure = cycles[0].type(cycles[0].props)
  expect(disclosure.type).toBe('details')
  expect(disclosure.props.open).toBeUndefined()
  const html = renderToStaticMarkup(disclosure)
  expect(html).toContain('2 files in import cycle')
  expect(html).toContain('Import evidence · 1')
  expect(html).toContain('service.py → other.py → service.py')
  expect(scoped).toBeUndefined()
  disclosure.props.children.at(-1).props.onClick()
  expect(scoped).toBe(view.cycles[0])
})

test('Report distinguishes no findings from unavailable metrics', () => {
  const view = fixture()
  view.cycles = []
  view.diagnostics = []
  view.details.nodes.find((node) => node.stableKey === 'run')!.properties = {}
  const html = renderToStaticMarkup(
    <GraphReport view={view} selected="run" {...actions} />,
  )
  expect(html).toContain('No diagnostics recorded')
  expect(html).toContain('No file-import cycles recorded')
  expect(html).toContain('Not available')
  expect(html).toContain('pass/fail results are not available')
})

test('Explore remains structural even with metrics and cycles available', () => {
  const view = fixture()
  const html = renderToStaticMarkup(
    <GraphSidebar
      graph={view.graph}
      details={view.details}
      selected="run"
      onSelect={actions.onSelect}
      onOpen={actions.onOpen}
    />,
  )
  expect(html).toContain('Connections')
  for (const text of [
    'Weighted complexity',
    'Dependency metrics',
    'Size metrics',
    'Circular dependencies',
    'Recorded findings',
  ])
    expect(html).not.toContain(text)
})

test('Report search, type filter, disclosure and explicit navigation work', () => {
  const script = `
    import { mock } from 'bun:test'
    const React = { ...await import('react') }
    const values = []; let cursor = 0
    mock.module('react', () => ({ ...React, useState: (initial) => {
      const index = cursor++; if (index === values.length) values.push(initial)
      return [values[index], value => values[index] = value]
    }}))
    const { ReportItems } = await import('./src/components/graph/report-items')
    const { renderToStaticMarkup } = await import('react-dom/server')
    const view = ${JSON.stringify(fixture())}
    let selected = null; let opened = null; let explored = null
    const props = { view, selected, onSelect: id => selected = id, onOpen: node => opened = node.id, onExplore: node => explored = node.id }
    const walk = (element, predicate) => {
      if (!element || typeof element !== 'object') return null
      if (predicate(element)) return element
      for (const child of [element.props?.children].flat(Infinity)) { const found = walk(child, predicate); if (found) return found }
      return null
    }
    const render = () => { cursor = 0; return ReportItems({ ...props, selected }) }
    let tree = render()
    walk(tree, e => e.props.placeholder)?.props.onChange({ target: { value: 'no-such-item' } })
    if (!renderToStaticMarkup(render()).includes('No matching items')) throw Error('Search failed')
    tree = render(); walk(tree, e => e.props.placeholder).props.onChange({ target: { value: 'run' } })
    tree = render(); walk(tree, e => e.type === 'select').props.onChange({ target: { value: 'file' } })
    if (!renderToStaticMarkup(render()).includes('No matching items')) throw Error('Type filter failed')
    tree = render(); walk(tree, e => e.type === 'select').props.onChange({ target: { value: 'all' } })
    tree = render()
    const group = walk(tree, e => e.props.title === 'In this scope')
    const record = group.type(group.props).props.children[1][0]
    let detail = record.type(record.props)
    detail.props.children[0].props.children[0].props.onClick({ preventDefault() {} })
    if (selected !== 'run') throw Error('Disclosure failed')
    detail = record.type({ ...record.props, selected })
    const reportItem = detail.props.children[0].props.children[1].props.children[0]
    const itemTree = reportItem.type(reportItem.props)
    walk(itemTree, e => e.props.onClick)?.props.onClick()
    if (explored !== 'run') throw Error('Explore link failed')
    console.log('PASS')
  `
  const result = Bun.spawnSync(['bun', '--eval', script], {
    cwd: process.cwd(),
  })
  expect(result.stderr.toString()).toBe('')
  expect(result.exitCode).toBe(0)
})
