import { eq } from 'drizzle-orm'
import { graphNodes } from './schema'
import type { AppDatabase } from './connection'
import type { NodeRepositoryPort } from '../../repository-ports'

export class NodeRepository implements NodeRepositoryPort {
  constructor(private readonly database: AppDatabase) {}

  read(id: string) {
    return this.database
      .select({
        stableKey: graphNodes.stableKey,
        parentStableKey: graphNodes.parentStableKey,
        type: graphNodes.type,
        kind: graphNodes.kind,
        level: graphNodes.level,
        name: graphNodes.name,
        language: graphNodes.language,
        qualifiedName: graphNodes.qualifiedName,
        path: graphNodes.path,
      })
      .from(graphNodes)
      .where(eq(graphNodes.repositoryId, id))
      .orderBy(graphNodes.ordinal)
      .all()
  }
}
