import { expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { fixtureDetails } from '@/lib/graph-details-test.helpers'
import { fixtureNodes } from '@/lib/project-graph-fixture'
import { buildProjectGraph } from '@/lib/project-graph'
import { ReportFindings } from './report-findings'
import { ReportItems } from './report-items'
import type { GraphView } from '@/lib/graph-data'

function fixture(): GraphView {
  const artifact = {
    schemaVersion: 2 as const,
    repository: { name: 'Example' },
    nodes: fixtureNodes(),
    edges: [],
  }
  return {
    state: 'ready',
    repositoryId: 'repo',
    importedAt: 0,
    rootName: 'Example',
    graphRevision: 'rev',
    scope: { kind: 'repository' },
    trail: [{ label: 'Example', scope: { kind: 'repository' } }],
    graph: buildProjectGraph(artifact, { kind: 'repository' }),
    details: fixtureDetails(artifact),
    relations: [],
    cycles: [],
    diagnostics: [],
  }
}

test('Recorded severity controls color; no findings does not imply a passing check', () => {
  const view = fixture()
  view.diagnostics = ['error', 'warning', 'info'].map((severity) => ({
    severity,
    path: 'source.py',
    language: 'python',
    message: `Recorded ${severity}`,
  }))
  const html = renderToStaticMarkup(
    <ReportFindings view={view} onCycleScope={() => {}} />,
  )
  for (const severity of ['error', 'warning', 'neutral'])
    expect(html).toContain(`report-diagnostic report-${severity}`)
  expect(html).toContain('Other issues may still exist')
  expect(html).not.toContain('All checks passed')
})

test('Collapsed folders offer separate report and graph actions without opening details', () => {
  const view = fixture()
  const html = renderToStaticMarkup(
    <ReportItems
      view={view}
      selected={null}
      onSelect={() => {}}
      onOpen={() => {}}
      onExplore={() => {}}
    />,
  )
  expect(html).toContain('Open report')
  expect(html).toContain('View in graph')
  expect(html).toContain('aria-label="Open report for ')
  expect(html).toContain('aria-label="View in graph: ')
  expect(html).toContain('report-row-actions')
  expect(html).not.toContain('report-item-detail')
  expect(html).not.toContain('report-error')
  expect(html).not.toContain('report-warning')
})

test('Collapsed report and graph actions use the same item without changing selection', () => {
  const script = `
    import { mock } from 'bun:test'
    const React = { ...await import('react') }
    mock.module('react', () => ({ ...React, useState: initial => [initial, () => {}] }))
    const { ReportItems } = await import('./src/components/graph/report-items')
    const view = ${JSON.stringify(fixture())}
    let opened, explored, selected = false
    const props = { view, selected: null, onSelect: () => selected = true, onOpen: item => opened = item, onExplore: item => explored = item }
    const tree = ReportItems(props)
    const children = tree.props.children
    const group = children.find(child => child?.props?.title === 'In this scope')
    const records = group.type(group.props).props.children[1]
    const record = records.find(record => view.graph.nodes.find(node => node.id === record.props.node.stableKey)?.scope)
    const row = record.type(record.props)
    if (row.props.children[0].props.open) throw Error('Row should start collapsed')
    const actions = row.props.children[1]
    const buttons = actions.type(actions.props).props.children
    const expected = view.graph.nodes.find(node => node.id === record.props.node.stableKey)
    buttons[0].props.onClick()
    if (opened !== expected || explored || selected) throw Error('Report action changed the wrong state')
    buttons[1].props.onClick()
    if (explored !== expected || selected) throw Error('Graph action changed the wrong state')
    if (buttons[0].props['aria-label'] !== 'Open report for ' + record.props.node.name) throw Error('Report label mismatch')
    if (buttons[1].props['aria-label'] !== 'View in graph: ' + record.props.node.name) throw Error('Graph label mismatch')
  `
  const result = Bun.spawnSync(['bun', '--eval', script], {
    cwd: process.cwd(),
  })
  expect(result.stderr.toString()).toBe('')
  expect(result.exitCode).toBe(0)
})
