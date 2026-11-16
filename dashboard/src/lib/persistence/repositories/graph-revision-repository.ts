import { eq, inArray } from 'drizzle-orm'
import { repositoryGraphs } from './schema'
import { selectedRows } from './selected-rows'
import type { AppDatabase } from './connection'
import type { GraphRevisionRepositoryPort } from '../../repository-ports'

export class GraphRevisionRepository implements GraphRevisionRepositoryPort {
  constructor(private readonly database: AppDatabase) {}

  find(id: string) {
    return this.database
      .select()
      .from(repositoryGraphs)
      .where(eq(repositoryGraphs.repositoryId, id))
      .get()
  }

  read(ids: Array<string>) {
    return selectedRows(ids, (batch) =>
      this.database
        .select()
        .from(repositoryGraphs)
        .where(inArray(repositoryGraphs.repositoryId, batch))
        .all(),
    )
  }
}
