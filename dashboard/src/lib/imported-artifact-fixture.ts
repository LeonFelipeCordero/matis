import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { rustFixture } from '../../tests/rust-cli'
import type { Artifact, ArtifactNode } from './architecture-artifact'

const samples = {
  'src/main/kotlin/api/ChannelApi.kt': `class ChannelApi {
    fun createChannel(): Int { return findByName() }
    fun deleteChannel(): Int { return findByName() }
    fun findByName(): Int { return 1 }
    fun addRoutingKey(): Int { return createChannel() }
}
`,
  'src/main/kotlin/jobs/Worker.kt': `class Worker {
    fun perform(): Int { return 1 }
}
`,
  'extensions/service/config.ex': `defmodule Sample.Config do
  def enabled(), do: true
end
`,
  'extensions/tools/index.ts': `export function ready() { return true }
`,
  'extensions/service/lib/application.ex': `defmodule Sample.Application do
  def start() do
    initialize()
  end
  def initialize() do
    :ok
  end
end
`,
  'extensions/tools/nested/helper.ts': `export function helper() { return 1 }
export function run() { return helper() }
`,
  'main.py': `def run():
    return 1
`,
}

let imported: Artifact | undefined

/** Projection-only fixture from the real Rust analyzer; no persisted JSON export. */
export function importedArtifact(): Artifact {
  if (imported) return imported
  const directory = mkdtempSync(join(tmpdir(), 'matis-language-fixture-'))
  const source = join(directory, 'source')
  try {
    for (const [path, content] of Object.entries(samples)) {
      const file = join(source, path)
      mkdirSync(dirname(file), { recursive: true })
      writeFileSync(file, content)
    }
    imported = JSON.parse(
      rustFixture(join(directory, 'unused.sqlite'), 'analyze', source),
    ) as Artifact
    return imported
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
}

/** Large, nested graph with exact sizes and dense direct observations. */
export function largeArtifact(): Artifact {
  const nodes: Array<ArtifactNode> = []
  const artifact: Artifact = {
    schemaVersion: 2,
    repository: { name: 'large-fixture' },
    nodes,
    edges: [],
    views: { fixture: { preserved: true } },
    stats: { nodes: 2022, edges: 8421 },
    diagnostics: [],
  }
  const add = (key: string, kind: string, parent?: ArtifactNode) => {
    const node: ArtifactNode = {
      stableKey: key,
      type: kind,
      kind,
      level:
        kind === 'directory'
          ? 'module'
          : kind === 'function'
            ? 'callable'
            : kind,
      name: key,
      path: key,
      language: kind === 'repository' ? null : 'typescript',
      parentStableKey: parent?.stableKey ?? null,
      ancestorStableKeys: parent
        ? [...parent.ancestorStableKeys, parent.stableKey]
        : [],
      depth: parent ? parent.depth + 1 : 0,
    }
    nodes.push(node)
    if (parent)
      artifact.edges.push({
        sourceStableKey: parent.stableKey,
        targetStableKey: key,
        type: 'CONTAINS',
        confidence: 'EXTRACTED',
      })
    return node
  }
  const root = add('repository', 'repository')
  const source = add('src', 'directory', root)
  for (let folder = 0; folder < 20; folder++)
    addFiles(add(`src/package-${folder}`, 'directory', source), add)
  const callables = nodes.filter((node) => node.level === 'callable')
  artifact.edges.push(
    ...callables.flatMap((callable, index) =>
      [1, 4, 80, 1599].map((offset) => ({
        sourceStableKey: callable.stableKey,
        targetStableKey:
          callables[(index + offset) % callables.length].stableKey,
        type: 'CALLS',
        confidence: 'EXTRACTED' as const,
      })),
    ),
  )
  return artifact
}

type AddNode = (
  key: string,
  kind: string,
  parent?: ArtifactNode,
) => ArtifactNode

function addFiles(parent: ArtifactNode, add: AddNode) {
  for (let file = 0; file < 20; file++)
    addFunctions(add(`${parent.path}/file-${file}.ts`, 'file', parent), add)
}

function addFunctions(owner: ArtifactNode, add: AddNode) {
  for (let index = 0; index < 4; index++)
    add(`${owner.path}:function-${index}`, 'function', owner)
}

export const artifactFixtures = {
  'CLI language samples': importedArtifact,
  'large nested fixture': largeArtifact,
}
