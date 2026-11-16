import { RepositoryError, requiredText } from './repository-error'
import type { RepositoryRepositoryPort } from '../repository-ports'
import type { GraphRevisionService } from './graph-revision-service'

export class RepositoryService {
  constructor(
    private readonly repositories: RepositoryRepositoryPort,
    private readonly revisions: GraphRevisionService,
  ) {}

  register(input: unknown) {
    if (!input || typeof input !== 'object' || Array.isArray(input))
      throw new RepositoryError(
        'Expected an object with name and location',
        400,
      )
    const data = input as Record<string, unknown>
    const name = requiredText(data.name, 'name', 200)
    const location = requiredText(data.location, 'location', 4096)
    const row = this.repositories.register({
      id: crypto.randomUUID(),
      name,
      location,
      createdAt: Date.now(),
    })
    if (!row)
      throw new RepositoryError(
        'A repository with this name already exists',
        409,
      )
    return { ...row, importedAt: null }
  }

  find(id: string) {
    return this.repositories.find(id)
  }

  list() {
    const rows = this.repositories.list()
    const revisions = this.revisions.read(rows.map((row) => row.id))
    return rows.map((row) => {
      const revision = revisions.get(row.id)
      return {
        ...row,
        importedAt:
          revision && revision.graphRevision !== null
            ? revision.importedAt
            : null,
      }
    })
  }
}
