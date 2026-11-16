import { and, eq, inArray } from 'drizzle-orm'
import { graphFiles } from './schema'
import { selectedRows } from './selected-rows'
import type { AppDatabase } from './connection'
import type { FileRepositoryPort } from '../../repository-ports'

export class FileRepository implements FileRepositoryPort {
  constructor(private readonly database: AppDatabase) {}

  read(id: string, keys: Array<string>) {
    const read = (batch: Array<string>) =>
      this.database
        .select({
          stableKey: graphFiles.stableKey,
          bytes: graphFiles.bytes,
          sha256: graphFiles.sha256,
        })
        .from(graphFiles)
        .where(
          and(
            eq(graphFiles.repositoryId, id),
            inArray(graphFiles.stableKey, batch),
          ),
        )
        .all()
    return selectedRows(keys, read)
  }
}
