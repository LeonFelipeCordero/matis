import {
  createRepositoryContext,
  repositoryDatabasePath,
} from './persistence/repositories/factory'
import { GraphScopeError } from './project-graph'
import { RepositoryError, requiredText } from './services/repository-error'
import type { GraphScope } from './project-graph-types'

export { RepositoryError } from './services/repository-error'

/** Public API facade; bootstrap composes services and owns connection lifetime. */
export function createRepositoryService(path?: string) {
  const repository = createRepositoryContext(path)
  return {
    registerRepository: (input: unknown) =>
      repository.registerRepository(input),
    listRepositories: () => repository.listRepositories(),
    getArchitecture(
      repositoryId: string,
      scope: GraphScope = { kind: 'repository' },
    ) {
      const id = requiredText(repositoryId, 'repositoryId', 200)
      const graph = repository.getArchitecture(id, scope)
      if (graph === undefined)
        throw new RepositoryError('Repository not found', 404)
      return graph
    },
    close: () => repository.close(),
  }
}

export type RepositoryService = ReturnType<typeof createRepositoryService>
export const defaultDatabasePath = () => repositoryDatabasePath()
let service: RepositoryService | undefined
const getService = () => (service ??= createRepositoryService())
export const registerRepository = (input: unknown) =>
  getService().registerRepository(input)
export const listRepositories = () => getService().listRepositories()
export const getArchitecture = (id: string, scope?: GraphScope) =>
  getService().getArchitecture(id, scope)

export function architectureScope(parameters: URLSearchParams): GraphScope {
  const kind = parameters.get('kind') ?? 'repository'
  if (kind === 'repository') return { kind }
  if (kind !== 'folder' && kind !== 'file')
    throw new RepositoryError('Invalid graph scope', 400)
  return { kind, key: requiredText(parameters.get('key'), 'key', 4096) }
}

export const repositoryErrorResponse = (error: unknown) => {
  if (error instanceof GraphScopeError)
    return Response.json({ error: error.message }, { status: 400 })
  if (error instanceof RepositoryError)
    return Response.json({ error: error.message }, { status: error.status })
  console.error(error)
  return Response.json({ error: 'Internal server error' }, { status: 500 })
}
