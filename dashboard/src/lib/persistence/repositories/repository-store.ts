import { eq } from 'drizzle-orm'
import { repositories } from './schema'
import type { AppDatabase } from './connection'
import type { RepositoryRepositoryPort } from '../../repository-ports'
import type { RepositoryRecord } from '../../graph-records'

export class RepositoryStore implements RepositoryRepositoryPort {
  constructor(private readonly database: AppDatabase) {}

  register(input: RepositoryRecord) {
    return this.database
      .insert(repositories)
      .values(input)
      .onConflictDoNothing()
      .returning()
      .all()
      .at(0)
  }

  find(id: string) {
    return this.database
      .select()
      .from(repositories)
      .where(eq(repositories.id, id))
      .get()
  }

  list() {
    return this.database
      .select()
      .from(repositories)
      .orderBy(repositories.createdAt, repositories.id)
      .all()
  }
}
