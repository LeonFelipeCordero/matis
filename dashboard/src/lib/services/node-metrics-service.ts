import type { NodeMetricsRepositoryPort } from '../repository-ports'

export class NodeMetricsService {
  constructor(private readonly metrics: NodeMetricsRepositoryPort) {}

  read(id: string, keys: Array<string>) {
    return new Map(
      this.metrics
        .read(id, keys)
        .map(({ stableKey, ...values }) => [
          stableKey,
          Object.fromEntries(
            Object.entries(values).filter(([, value]) => value !== null),
          ),
        ]),
    )
  }

  complexities(id: string, keys: Array<string>) {
    return this.metrics.read(id, keys).map((row) => ({
      stableKey: row.stableKey,
      total: row.cyclomaticComplexity,
    }))
  }
}
