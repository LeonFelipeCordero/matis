import type { DiagnosticRepositoryPort } from '../repository-ports'

export class DiagnosticService {
  constructor(private readonly diagnostics: DiagnosticRepositoryPort) {}

  read(id: string) {
    return this.diagnostics.read(id)
  }
}
