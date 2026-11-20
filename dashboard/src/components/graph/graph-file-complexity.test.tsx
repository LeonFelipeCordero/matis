import { expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { sumFileComplexity } from '@/lib/file-complexity'
import { buildProjectGraph } from '@/lib/project-graph'
import { GraphFileComplexity } from './graph-file-complexity'
import { GraphReport } from './graph-report'
import { reportView } from './graph-report-test.helpers'
import type { Artifact, ArtifactNode } from '@/lib/architecture-artifact'
import type { GraphScope, ProjectGraph } from '@/lib/project-graph'

type ComplexityNode = {
  id: string
  type: string
  parent: string | null
  value?: number
}

function fixture(): Artifact {
  const nodes: Array<ArtifactNode> = []
  function add({ id, type, parent, value }: ComplexityNode) {
    const owner = nodes.find((node) => node.stableKey === parent)
    nodes.push({
      stableKey: id,
      name: id,
      type,
      kind: type,
      level: ['function', 'method', 'constructor'].includes(type)
        ? 'callable'
        : type,
      path: 'shared.ts',
      language: 'typescript',
      parentStableKey: parent,
      ancestorStableKeys: owner
        ? [...owner.ancestorStableKeys, owner.stableKey]
        : [],
      depth: owner ? owner.depth + 1 : 0,
      properties:
        value === undefined ? undefined : { cyclomaticComplexity: value },
    })
  }
  add({ id: 'repo', type: 'repository', parent: null })
  add({ id: 'file', type: 'file', parent: 'repo' })
  add({ id: 'class', type: 'class', parent: 'file', value: 900 })
  add({ id: 'method', type: 'method', parent: 'class', value: 8 })
  add({ id: 'nested', type: 'function', parent: 'method', value: 2 })
  add({ id: 'function', type: 'function', parent: 'file', value: 3 })
  add({ id: 'constructor', type: 'constructor', parent: 'class', value: 100 })
  add({ id: 'other-file', type: 'file', parent: 'repo' })
  add({ id: 'outside', type: 'function', parent: 'other-file', value: 50 })
  add({ id: 'empty', type: 'file', parent: 'repo' })
  return {
    schemaVersion: 2,
    repository: { name: 'Example' },
    nodes,
    edges: [
      {
        sourceStableKey: 'method',
        targetStableKey: 'outside',
        type: 'CALLS',
        confidence: 'EXTRACTED',
      },
    ],
  }
}

function summary(artifact: Artifact, fileKey = 'file') {
  return renderToStaticMarkup(
    <GraphFileComplexity sum={sumFileComplexity(artifact, fileKey)} />,
  )
}

type ReportSelection = {
  artifact: Artifact
  selected: string | null
  scope?: GraphScope
  graph?: ProjectGraph
}

function selectedFile({
  artifact,
  selected,
  scope = { kind: 'repository' },
  graph = buildProjectGraph(artifact, scope),
}: ReportSelection) {
  return renderToStaticMarkup(
    <GraphReport
      view={{ ...reportView(artifact, scope), graph }}
      selected={selected}
      onSelect={() => {}}
      onOpen={() => {}}
      onExplore={() => {}}
      onCycleScope={() => {}}
    />,
  )
}

test('sums owned functions and methods once, not classes, constructors, or dependencies', () => {
  const artifact = fixture()
  const snapshot = structuredClone(artifact)
  expect(sumFileComplexity(artifact, 'file')).toEqual({
    total: 13,
    measured: 3,
    functions: 3,
  })
  expect(sumFileComplexity(artifact, 'other-file')).toEqual({
    total: 50,
    measured: 1,
    functions: 1,
  })
  expect(artifact).toEqual(snapshot)
  expect(
    artifact.nodes.find((node) => node.stableKey === 'file')!.properties,
  ).toBeUndefined()
})

test('selected file report and metric component show the same sum', () => {
  const artifact = fixture()
  for (const html of [
    selectedFile({ artifact, selected: 'file' }),
    summary(artifact),
  ]) {
    expect(html).toContain('aria-label="File weighted complexity sum"')
    expect(html).toContain('<h4>Weighted complexity sum</h4>')
    expect(html).toContain('<strong>13</strong>')
    expect(html).toContain(
      'Sum of weighted complexity across 3 functions/methods in this file',
    )
    expect(html).not.toContain('Sum of total complexity')
  }
  expect(selectedFile({ artifact, selected: 'empty' })).toContain(
    '<strong>0</strong>',
  )
  expect(selectedFile({ artifact, selected: 'empty' })).toContain(
    '0 functions/methods',
  )
})

test('file metric component retains the current file sum', () => {
  const html = summary(fixture())
  expect(html).toContain('Weighted complexity sum')
  expect(html).toContain('<strong>13</strong>')
})

test('report shows file sums only after selection, independent of boundary nodes', () => {
  const artifact = fixture()
  const graph = buildProjectGraph(artifact, { kind: 'repository' })
  expect(selectedFile({ artifact, selected: null })).not.toContain(
    'Weighted complexity sum',
  )
  graph.nodes = graph.nodes.filter((node) => node.id !== 'empty')
  graph.nodes.find((node) => node.id === 'other-file')!.role = 'boundary'
  const html = selectedFile({
    artifact,
    selected: 'file',
    scope: { kind: 'repository' },
    graph,
  })
  expect(html).toContain('Weighted complexity sum')
  expect(html).toContain('<strong>13</strong>')
  expect(html).not.toContain('file-complexity-value"><strong>50</strong>')
})

test('outside-scope file inspector uses the target file sum', () => {
  const artifact = fixture()
  const graph = buildProjectGraph(artifact, { kind: 'file', key: 'file' })
  expect(graph.nodes.find((node) => node.id === 'outside')?.role).toBe(
    'boundary',
  )
  // In a folder view the same target is represented by its owning file.
  expect(selectedFile({ artifact, selected: 'other-file' })).toContain(
    '<strong>50</strong>',
  )
})

test('partial and legacy data are not presented as a complete sum or zero', () => {
  const artifact = fixture()
  artifact.nodes.find((node) => node.stableKey === 'nested')!.properties =
    undefined
  expect(sumFileComplexity(artifact, 'file')).toEqual({
    total: 11,
    measured: 2,
    functions: 3,
  })
  const partial = summary(artifact)
  expect(partial).toContain('<strong>11</strong>')
  expect(partial).toContain('Partial sum')
  expect(partial).toContain('2 of 3 functions/methods measured')
  for (const node of artifact.nodes) node.properties = undefined
  const legacy = summary(artifact)
  expect(legacy).toContain('Not available')
  expect(legacy).not.toContain('<strong>0</strong>')
  expect(legacy).toContain('Re-index and import')
})
