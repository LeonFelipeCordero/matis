import type { GraphRevisionRepositoryPort } from '../repository-ports'

export class GraphRevisionService {
  constructor(private readonly revisions: GraphRevisionRepositoryPort) {}

  find(id: string) {
    return this.revisions.find(id)
  }

  read(ids: Array<string>) {
    return new Map(
      this.revisions.read(ids).map((row) => [row.repositoryId, row]),
    )
  }
}
