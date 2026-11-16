import { and, eq, inArray } from 'drizzle-orm'
import { graphMetrics } from './schema'
import { selectedRows } from './selected-rows'
import type { AppDatabase } from './connection'
import type { NodeMetricsRepositoryPort } from '../../repository-ports'

export class NodeMetricsRepository implements NodeMetricsRepositoryPort {
  constructor(private readonly database: AppDatabase) {}

  read(id: string, keys: Array<string>) {
    const read = (batch: Array<string>) =>
      this.database
        .select({
          stableKey: graphMetrics.stableKey,
          lineCount: graphMetrics.lineCount,
          parameterCount: graphMetrics.parameterCount,
          baseCyclomaticComplexity: graphMetrics.baseCyclomaticComplexity,
          nestedCyclomaticComplexity: graphMetrics.nestedCyclomaticComplexity,
          loopNestedComplexity: graphMetrics.loopNestedComplexity,
          cyclomaticComplexity: graphMetrics.cyclomaticComplexity,
          maxDecisionDepth: graphMetrics.maxDecisionDepth,
        })
        .from(graphMetrics)
        .where(
          and(
            eq(graphMetrics.repositoryId, id),
            inArray(graphMetrics.stableKey, batch),
          ),
        )
        .all()
    return selectedRows(keys, read)
  }
}
