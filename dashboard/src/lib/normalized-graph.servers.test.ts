import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, expect, test } from 'bun:test'
import { Database } from 'bun:sqlite'
import { rustCli, rustFixture } from '../../tests/rust-cli'
import { migratedDatabaseFixture } from '../../tests/migrated-database-fixture'
import {
  architectureScope,
  createRepositoryService,
  repositoryErrorResponse,
} from './repositories.servers'
import { buildProjectGraph, getGraphTrail } from './project-graph'
import { hideTestFiles } from './graph-visibility'
import { projectCycles } from './graph-cycles'
import { sumFileComplexity } from './file-complexity'
import type { RepositoryService } from './repositories.servers'
import type {
  Artifact,
  ArtifactNode,
  DirectEdge,
} from './architecture-artifact'
import type { GraphScope } from './project-graph-types'
import type { GraphView } from './graph-data'

let directory: string
let path: string
let database: Database
let service: RepositoryService
const copyMigratedDatabase = migratedDatabaseFixture()
beforeEach(() => {
  directory = mkdtempSync(join(tmpdir(), 'normalized-dashboard-'))
  path = join(directory, 'matis.sqlite')
  copyMigratedDatabase(path)
  database = new Database(path, {
    readwrite: true,
    create: false,
    strict: true,
  })
  service = createRepositoryService(path)
})
afterEach(() => {
  service.close()
  database.close()
  rmSync(directory, { recursive: true, force: true })
})
function ready(id: string, scope: GraphScope = { kind: 'repository' }) {
  const view = service.getArchitecture(id, scope)
  if (view?.state !== 'ready') throw new Error('Expected normalized graph')
  return view
}
function save(artifact: Artifact, name = 'custom'): string {
  const fixturePath = join(directory, 'fixture.json')
  writeFileSync(fixturePath, JSON.stringify(artifact))
  return JSON.parse(rustFixture(path, 'save', fixturePath, name)) as string
}
type FixtureNode = {
  key: string
  type: string
  level: string
  parent: string | null
}

function fixture(): Artifact {
  const nodes: Array<ArtifactNode> = []
  const add = ({ key, type, level, parent }: FixtureNode) => {
    const owner = nodes.find((node) => node.stableKey === parent)
    const node: ArtifactNode = {
      stableKey: key,
      name: key,
      type,
      kind: type,
      level,
      path: key === 'repo' ? '.' : key,
      parentStableKey: parent,
      language: 'python',
      ancestorStableKeys: owner
        ? [...owner.ancestorStableKeys, owner.stableKey]
        : [],
      depth: owner ? owner.depth + 1 : 0,
    }
    nodes.push(node)
    return node
  }
  add({ key: 'repo', type: 'repository', level: 'repository', parent: null })
  add({ key: 'a.py', type: 'file', level: 'file', parent: 'repo' })
  add({ key: 'b.py', type: 'file', level: 'file', parent: 'repo' })
  add({ key: 'test_a.py', type: 'file', level: 'file', parent: 'repo' })
  add({ key: 'C', type: 'class', level: 'type', parent: 'a.py' })
  const run = add({
    key: 'run',
    type: 'method',
    level: 'callable',
    parent: 'C',
  })
  run.signature = 'run(self)'
  run.startLine = 3
  run.startColumn = 0
  run.endLine = 4
  run.endColumn = 12
  run.properties = {
    lineCount: 2,
    parameterCount: 0,
    cyclomaticComplexity: 1,
    nestedCyclomaticComplexity: 0,
    decorators: ['second', 'first'],
    visibility: 'public',
  }
  add({ key: 'nested', type: 'function', level: 'callable', parent: 'run' })
  add({
    key: 'constructor',
    type: 'constructor',
    level: 'callable',
    parent: 'C',
  }).properties = {
    cyclomaticComplexity: 9,
  }
  const dependency: DirectEdge = {
    sourceStableKey: 'run',
    targetStableKey: 'b.py',
    type: 'CALLS',
    confidence: 'INFERRED',
    path: 'a.py',
    line: 4,
    evidence: 'b.run()',
    typeOnly: true,
  }
  return {
    schemaVersion: 2,
    repository: { name: 'custom' },
    nodes,
    edges: [
      dependency,
      {
        ...dependency,
        confidence: 'EXTRACTED',
        evidence: 'second observation',
      },
    ],
    diagnostics: [
      {
        severity: 'warning',
        path: 'z.py',
        language: 'python',
        message: 'first',
      },
      {
        severity: 'error',
        path: 'a.py',
        language: 'python',
        message: 'second',
      },
    ],
  }
}

function expectPartialDetails(view: GraphView) {
  expect(view.details.nodes.map((node) => node.stableKey)).toEqual([
    'a.py',
    'b.py',
    'run',
    'nested',
    'constructor',
  ])
  expect(
    view.details.nodes.find((node) => node.stableKey === 'run'),
  ).toMatchObject({
    signature: 'run(self)',
    startLine: 3,
    startColumn: 0,
    endLine: 4,
    endColumn: 12,
    properties: {
      lineCount: 2,
      parameterCount: 0,
      cyclomaticComplexity: 1,
      nestedCyclomaticComplexity: 0,
      instability: null,
      decorators: ['second', 'first'],
      visibility: 'public',
    },
  })
  expect(
    view.details.nodes.find((node) => node.stableKey === 'a.py')!.properties,
  ).toMatchObject({ fanIn: 1, fanOut: 2, instability: 2 / 3 })
  expect(
    view.details.nodes.find((node) => node.stableKey === 'b.py')!.properties,
  ).toMatchObject({ fanIn: 1, fanOut: 1, instability: 0.5 })
  const nested = view.details.nodes.find((node) => node.stableKey === 'nested')!
  expect(nested).toMatchObject({
    signature: null,
    startColumn: null,
    endColumn: null,
  })
  expect(nested.properties).not.toHaveProperty('signature')
  expect(nested.properties).not.toHaveProperty('startColumn')
  expect(nested.properties).not.toHaveProperty('endColumn')
  expect(nested.properties).not.toHaveProperty('cyclomaticComplexity')
  expect(view.details.fileComplexities['a.py']).toEqual({
    total: 1,
    measured: 1,
    functions: 2,
  })
  expect(view.details.fileComplexities['b.py']).toEqual({
    total: 0,
    measured: 0,
    functions: 0,
  })
}

function expectMetrics(node: ArtifactNode, original: ArtifactNode) {
  for (const key of [
    'fanIn',
    'fanOut',
    'instability',
    'transitiveChangeImpact',
    'lineCount',
    'parameterCount',
    'cyclomaticComplexity',
    'nestedCyclomaticComplexity',
  ])
    expect(node.properties?.[key]).toEqual(original.properties?.[key])
}

function expectAnalyzedDetails(view: GraphView, artifact: Artifact) {
  for (const node of view.details.nodes) {
    const original = artifact.nodes.find(
      (entry) => entry.stableKey === node.stableKey,
    )!
    expectMetrics(node, original)
    if (node.level === 'file')
      expect(view.details.fileComplexities[node.stableKey]).toEqual(
        sumFileComplexity(artifact, node.stableKey),
      )
  }
}

function enqueueScopes(graph: GraphView['graph'], pending: Array<GraphScope>) {
  for (const node of graph.nodes)
    if (
      node.scope &&
      !pending.some(
        (entry) => JSON.stringify(entry) === JSON.stringify(node.scope),
      )
    )
      pending.push(node.scope)
}

test('normalized-only DTOs preserve partial metrics, symbol details, ordered diagnostics and duplicate evidence', () => {
  const artifact = fixture()
  const expectedRelations = structuredClone(artifact.edges)
  artifact.nodes.push({
    ...artifact.nodes[0],
    stableKey: 'external',
    type: 'external',
    level: 'external',
    parentStableKey: 'repo',
    ancestorStableKeys: ['repo'],
    depth: 1,
  })
  artifact.nodes.push({
    ...artifact.nodes[1],
    stableKey: 'external-child.py',
    parentStableKey: 'external',
    ancestorStableKeys: ['repo', 'external'],
    depth: 2,
  })
  artifact.edges.push(
    {
      ...artifact.edges[0],
      sourceStableKey: 'external-child.py',
      targetStableKey: 'a.py',
    },
    { ...artifact.edges[0], targetStableKey: 'external-child.py' },
    {
      sourceStableKey: 'b.py',
      targetStableKey: 'a.py',
      type: 'CALLS',
      confidence: 'EXTRACTED',
    },
    {
      sourceStableKey: 'run',
      targetStableKey: 'test_a.py',
      type: 'CALLS',
      confidence: 'EXTRACTED',
    },
    {
      sourceStableKey: 'run',
      targetStableKey: 'external',
      type: 'IMPORTS',
      confidence: 'EXTRACTED',
    },
  )
  artifact.diagnostics!.push({
    severity: 'warning',
    path: 'a.py',
    language: 'python',
    message: 'Parse warning',
  })
  const id = save(artifact)
  const other = fixture()
  other.nodes.find((node) => node.stableKey === 'run')!.name =
    'other-repository'
  const otherId = save(other, 'other')
  const view = ready(id, { kind: 'file', key: 'a.py' })
  expect(view.relations).toEqual(expectedRelations)
  expect(view.graph.nodes.map((node) => node.id)).toEqual([
    'run',
    'nested',
    'constructor',
    'b.py',
  ])
  expect(view.graph.edges).toMatchObject([
    { source: 'run', target: 'b.py', count: 2, relationships: { CALLS: 2 } },
  ])
  expectPartialDetails(view)
  expect(view.diagnostics).toEqual(artifact.diagnostics ?? [])
  expect(
    ready(otherId, { kind: 'file', key: 'a.py' }).graph.nodes[0].label,
  ).toBe('other-repository')
  expect(ready(id).graph.nodes.some((node) => node.id === 'test_a.py')).toBe(
    false,
  )
  expect(() => ready(id, { kind: 'file', key: 'test_a.py' })).toThrow(
    'Unknown graph scope',
  )
  expect(
    database
      .query<{ name: string }, []>('PRAGMA table_info(repository_graphs)')
      .all()
      .map((row) => row.name),
  ).not.toContain('artifact')
})

test('Rust analyzer and normalized Bun reads agree on all navigable views, dependency metrics and cycles', () => {
  const source = join(directory, 'source')
  mkdirSync(source)
  writeFileSync(
    join(source, 'a.py'),
    'import b\nclass C:\n    def run(self):\n        return b.work()\n',
  )
  writeFileSync(join(source, 'b.py'), 'import a\ndef work():\n    return 1\n')
  writeFileSync(
    join(source, 'test_a.py'),
    'import a\ndef test_run():\n    return a.C().run()\n',
  )
  writeFileSync(join(source, 'bad.py'), 'def broken(:\n')
  const { repositoryId: id } = JSON.parse(rustCli(path, 'index', '-f', source))
  const expected = JSON.parse(rustFixture(path, 'analyze', source)) as Artifact
  const visible = hideTestFiles(expected)
  const pending: Array<GraphScope> = [{ kind: 'repository' }]
  for (const scope of pending) {
    const view = ready(id, scope)
    const graph = buildProjectGraph(visible, scope)
    expect(view.graph.edges).toEqual(graph.edges)
    expect(view.graph.nodes.map(({ properties, ...node }) => node)).toEqual(
      graph.nodes.map(({ properties, ...node }) => node),
    )
    expect(view.trail).toEqual(getGraphTrail(visible, scope))
    expect(view.cycles.map((cycle) => cycle.group)).toEqual(
      projectCycles(expected, graph, scope)!.map((cycle) => cycle.group),
    )
    expect(view.diagnostics).toEqual(expected.diagnostics ?? [])
    expectAnalyzedDetails(view, expected)
    enqueueScopes(graph, pending)
  }
  expect(pending.length).toBeGreaterThan(2)
  expect(ready(id).cycles).toHaveLength(1)
})

test('type-only imports do not create cycles; hidden membership removes an entire cyclic group', () => {
  const artifact = fixture()
  const edge = (
    source: string,
    target: string,
    typeOnly = false,
  ): DirectEdge => ({
    sourceStableKey: source,
    targetStableKey: target,
    type: 'IMPORTS',
    confidence: 'EXTRACTED',
    typeOnly,
  })
  artifact.edges = [edge('a.py', 'b.py'), edge('b.py', 'a.py', true)]
  expect(ready(save(artifact)).cycles).toEqual([])
  artifact.edges[1].typeOnly = false
  const id = save(artifact, 'runtime-imports')
  expect(ready(id).cycles[0].group.nodeStableKeys).toEqual(['a.py', 'b.py'])
  artifact.edges.push(edge('a.py', 'test_a.py'), edge('test_a.py', 'a.py'))
  expect(ready(save(artifact, 'hidden-member')).cycles).toEqual([])
})

test('cycle IDs, members and evidence ordering match Rust for Unicode stable keys', () => {
  const artifact = fixture()
  const keys = ['\ufffd.py', '😀.py']
  const renamed = new Map([
    ['a.py', keys[0]],
    ['b.py', keys[1]],
  ])
  for (const node of artifact.nodes) {
    node.stableKey = renamed.get(node.stableKey) ?? node.stableKey
    node.parentStableKey =
      renamed.get(node.parentStableKey ?? '') ?? node.parentStableKey
  }
  artifact.edges = [
    {
      sourceStableKey: keys[0],
      targetStableKey: keys[1],
      type: 'IMPORTS',
      confidence: 'EXTRACTED',
      path: 'é.py',
      line: 2,
      evidence: '😀',
    },
    {
      sourceStableKey: keys[0],
      targetStableKey: keys[1],
      type: 'IMPORTS',
      confidence: 'EXTRACTED',
      path: 'é.py',
      line: 10,
      evidence: '😀',
    },
    {
      sourceStableKey: keys[1],
      targetStableKey: keys[0],
      type: 'IMPORTS',
      confidence: 'INFERRED',
      path: null,
      line: null,
      evidence: null,
    },
  ]
  const fixturePath = join(directory, 'cycles.json')
  writeFileSync(fixturePath, JSON.stringify(artifact))
  const expected = JSON.parse(rustFixture(path, 'cycles', fixturePath))
  expect(ready(save(artifact)).cycles.map((cycle) => cycle.group)).toEqual(
    expected,
  )
})

test('ORM key batches preserve large scoped details, decorators, edge order and file sums', () => {
  const artifact = fixture()
  const extra = Array.from({ length: 510 }, (_, index): ArtifactNode => ({
    stableKey: `extra:${index}`,
    name: `extra${index}`,
    type: 'function',
    kind: 'function',
    level: 'callable',
    path: 'a.py',
    language: 'python',
    parentStableKey: 'a.py',
    ancestorStableKeys: ['repo', 'a.py'],
    depth: 2,
    properties: {
      cyclomaticComplexity: index % 3,
      decorators: ['second', 'first'],
    },
  }))
  artifact.nodes.push(...extra)
  const edges = extra
    .slice()
    .reverse()
    .map((node): DirectEdge => ({
      sourceStableKey: node.stableKey,
      targetStableKey: 'b.py',
      type: 'CALLS',
      confidence: 'EXTRACTED',
    }))
  artifact.edges.push(...edges)
  const id = save(artifact)
  const view = ready(id, { kind: 'file', key: 'a.py' })
  expect(
    view.details.nodes.filter((node) => node.stableKey.startsWith('extra:')),
  ).toHaveLength(510)
  expect(
    view.details.nodes.find((node) => node.stableKey === 'extra:509')
      ?.properties,
  ).toMatchObject({
    cyclomaticComplexity: 2,
    decorators: ['second', 'first'],
  })
  expect(view.relations.map((edge) => edge.sourceStableKey)).toEqual([
    'run',
    'run',
    ...edges.map((edge) => edge.sourceStableKey),
  ])
  expect(view.details.fileComplexities['a.py']).toEqual({
    functions: 512,
    measured: 511,
    total: 511,
  })
})

test('valid normalized empty graphs differ from unindexed rows and invalid scopes return 400', () => {
  const id = save({
    schemaVersion: 2,
    repository: { name: 'empty' },
    nodes: [],
    edges: [],
  })
  expect(ready(id)).toMatchObject({
    graph: { nodes: [], edges: [] },
    cycles: [],
  })
  let error: unknown
  try {
    ready(id, { kind: 'folder', key: 'missing' })
  } catch (caught) {
    error = caught
  }
  expect(repositoryErrorResponse(error).status).toBe(400)
  expect(database.inTransaction).toBe(false)
  expect(() => architectureScope(new URLSearchParams('kind=callable'))).toThrow(
    'Invalid graph scope',
  )
  expect(() => architectureScope(new URLSearchParams('kind=file'))).toThrow(
    'key is required',
  )
})
