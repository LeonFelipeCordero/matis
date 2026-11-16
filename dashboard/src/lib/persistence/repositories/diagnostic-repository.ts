import { eq } from 'drizzle-orm'
import { graphDiagnostics } from './schema'
import type { AppDatabase } from './connection'
import type { DiagnosticRepositoryPort } from '../../repository-ports'

export class DiagnosticRepository implements DiagnosticRepositoryPort {
  constructor(private readonly database: AppDatabase) {}

  read(id: string) {
    return this.database
      .select({
        severity: graphDiagnostics.severity,
        path: graphDiagnostics.path,
        language: graphDiagnostics.language,
        message: graphDiagnostics.message,
      })
      .from(graphDiagnostics)
      .where(eq(graphDiagnostics.repositoryId, id))
      .orderBy(graphDiagnostics.ordinal)
      .all()
  }
}
