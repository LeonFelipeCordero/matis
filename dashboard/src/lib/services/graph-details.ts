import { dependencyMetrics, levelOwners } from '../graph-analysis'
import type { ArtifactNode } from '../architecture-artifact'
import type { FileComplexitySum } from '../file-complexity'
import type { GraphDetails } from '../graph-data'
import type { NodeService } from './node-service'
import type { RelationshipService } from './relationship-service'
import type { NodeMetricsService } from './node-metrics-service'

export function graphDetails(
  id: string,
  keys: Array<string>,
  topology: Array<ArtifactNode>,
  services: {
    nodes: NodeService
    relationships: RelationshipService
    metrics: NodeMetricsService
  },
): GraphDetails {
  const details = services.nodes.details(id, keys)
  const measurements = services.metrics.read(id, keys)
  const dependencies = dependencyMetrics(
    topology,
    services.relationships.relations(id),
  )
  const keySet = new Set(keys)
  return {
    nodes: topology
      .filter((node) => keySet.has(node.stableKey))
      .map((node) => ({
        ...node,
        ...details.get(node.stableKey),
        properties: {
          ...measurements.get(node.stableKey),
          ...details.get(node.stableKey)?.properties,
          ...dependencies.get(node.stableKey),
        },
      })),
    fileComplexities: fileComplexities(id, keySet, topology, services.metrics),
  }
}

function fileComplexities(
  id: string,
  keys: Set<string>,
  topology: Array<ArtifactNode>,
  metrics: NodeMetricsService,
): Record<string, FileComplexitySum> {
  const owners = levelOwners(topology, 'file')
  const sums = new Map(
    topology
      .filter((node) => keys.has(node.stableKey) && node.level === 'file')
      .map((node) => [node.stableKey, { functions: 0, measured: 0, total: 0 }]),
  )
  const functions = topology
    .filter((node) => ['function', 'method'].includes(node.type))
    .map((node) => ({
      node,
      file: owners.get(node.stableKey),
    }))
    .filter(({ file }) => file !== undefined && sums.has(file))
  const measurements = new Map(
    metrics
      .complexities(
        id,
        functions.map(({ node }) => node.stableKey),
      )
      .map((row) => [row.stableKey, row.total]),
  )
  for (const { node, file } of functions) {
    const sum = sums.get(file!)!
    const total = measurements.get(node.stableKey)
    sum.functions += 1
    if (total !== null && total !== undefined) {
      sum.measured += 1
      sum.total += total
    }
  }
  return Object.fromEntries(sums)
}
