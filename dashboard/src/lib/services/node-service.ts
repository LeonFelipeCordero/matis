import type {
  DecoratorRepositoryPort,
  FileRepositoryPort,
  NodeRepositoryPort,
  SymbolRepositoryPort,
} from '../repository-ports'
import type { ArtifactNode } from '../architecture-artifact'

type TopologyNode = Omit<ArtifactNode, 'ancestorStableKeys' | 'depth'>

function hierarchy(rows: Array<TopologyNode>): Array<ArtifactNode> {
  const lookup = new Map(rows.map((row) => [row.stableKey, row]))
  return rows.map((row) => {
    const ancestors: Array<string> = []
    let parent = row.parentStableKey
    while (parent !== null) {
      ancestors.push(parent)
      parent = lookup.get(parent)!.parentStableKey
    }
    return {
      ...row,
      ancestorStableKeys: ancestors.reverse(),
      depth: ancestors.length,
    }
  })
}

export class NodeService {
  constructor(
    private readonly nodes: NodeRepositoryPort,
    private readonly files: FileRepositoryPort,
    private readonly symbols: SymbolRepositoryPort,
    private readonly decorators: DecoratorRepositoryPort,
  ) {}

  topology(id: string) {
    const rows = this.nodes.read(id)
    const symbols = new Map(
      this.symbols.read(id).map((row) => [row.stableKey, row]),
    )
    return hierarchy(
      rows.map((row) => ({
        ...row,
        startLine: symbols.get(row.stableKey)?.startLine ?? null,
        endLine: symbols.get(row.stableKey)?.endLine ?? null,
      })),
    )
  }

  details(id: string, keys: Array<string>) {
    const files = new Map(
      this.files.read(id, keys).map((row) => [row.stableKey, row]),
    )
    const symbols = new Map(
      this.symbols.read(id, keys).map((row) => [row.stableKey, row]),
    )
    const decorators = new Map<string, Array<string>>()
    for (const row of this.decorators.read(id, keys)) {
      const list = decorators.get(row.stableKey) ?? []
      list.push(row.decorator)
      decorators.set(row.stableKey, list)
    }
    return new Map(
      keys.map((key) => {
        const symbol = symbols.get(key)
        const file = files.get(key)
        const list = decorators.get(key)
        return [
          key,
          {
            signature: symbol?.signature ?? null,
            startColumn: symbol?.startColumn ?? null,
            endColumn: symbol?.endColumn ?? null,
            properties: {
              ...(symbol?.visibility != null
                ? { visibility: symbol.visibility }
                : {}),
              ...(file?.bytes != null ? { bytes: file.bytes } : {}),
              ...(file?.sha256 != null ? { sha256: file.sha256 } : {}),
              ...(list ? { decorators: list } : {}),
            },
          },
        ]
      }),
    )
  }
}
