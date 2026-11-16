import { and, eq, inArray } from 'drizzle-orm'
import { graphDecorators } from './schema'
import { selectedRows } from './selected-rows'
import type { AppDatabase } from './connection'
import type { DecoratorRepositoryPort } from '../../repository-ports'

export class DecoratorRepository implements DecoratorRepositoryPort {
  constructor(private readonly database: AppDatabase) {}

  read(id: string, keys: Array<string>) {
    const read = (batch: Array<string>) =>
      this.database
        .select({
          stableKey: graphDecorators.stableKey,
          decorator: graphDecorators.decorator,
        })
        .from(graphDecorators)
        .where(
          and(
            eq(graphDecorators.repositoryId, id),
            inArray(graphDecorators.stableKey, batch),
          ),
        )
        .orderBy(graphDecorators.stableKey, graphDecorators.ordinal)
        .all()
    return selectedRows(keys, read)
  }
}
