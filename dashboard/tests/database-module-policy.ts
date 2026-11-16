import { posix } from 'node:path'
import ts from 'typescript-ast'
import {
  drivers,
  factory,
  factoryNames,
  infrastructure,
  rawOrmNames,
  repositoryRoot,
  serviceFile,
} from './database-policy'
import {
  capabilityFiles,
  checkTableOwnership,
} from './database-ownership-policy'
import type { Reporter } from './database-syntax'

function modulePath(file: string, name: string) {
  return name.startsWith('@/')
    ? posix.normalize(`src/${name.slice(2)}`)
    : name.startsWith('.')
      ? posix.normalize(posix.join(posix.dirname(file), name))
      : name
}
function permittedFactory(node: ts.Node, resolved: string) {
  const clause = ts.isImportDeclaration(node.parent)
    ? node.parent.importClause
    : undefined
  const named = clause?.namedBindings
  if (resolved.replace(/\.ts$/, '') !== factory || clause?.name || !named)
    return false
  return (
    ts.isNamedImports(named) &&
    named.elements.every((binding) =>
      factoryNames.has((binding.propertyName ?? binding.name).text),
    )
  )
}
type ModuleContext = { file: string; name: string; report: Reporter }
function checkRepositoryModule(node: ts.Node, context: ModuleContext) {
  const { file, name, report } = context
  const resolved = modulePath(file, name).replace(/\.[cm]?[jt]sx?$/, '')
  checkTableOwnership(node, file, resolved, report)
  const repositoryModule =
    resolved.includes(repositoryRoot) ||
    resolved.endsWith(repositoryRoot.slice(0, -1))
  checkPortModule(node, { file, name: resolved, report })
  if (repositoryModule)
    checkServiceFactory(node, { file, name: resolved, report })
  if (
    repositoryModule ||
    /(?:database|(?:graph|repository)-store)(?:\.servers)?(?:\.ts)?$/.test(
      resolved,
    )
  )
    checkRepositoryOwner(node, context)
  if (ts.isExportDeclaration(node.parent) && repositoryModule)
    report(node, 'repository reexport')
}
function checkServiceFactory(
  node: ts.Node,
  { file, name, report }: ModuleContext,
) {
  if (file === serviceFile && !permittedFactory(node, name))
    report(node, 'service may import only exact domain repository factory')
}
function checkPortModule(node: ts.Node, { file, name, report }: ModuleContext) {
  if (
    /(?:^|\/)repository-ports(?:\.ts)?$/.test(name) &&
    !file.startsWith(repositoryRoot) &&
    file !== serviceFile &&
    !capabilityFiles.has(file)
  )
    report(node, 'repository port dependency outside service/repository')
}
function checkRepositoryOwner(node: ts.Node, { file, report }: ModuleContext) {
  if (!file.startsWith(repositoryRoot) && file !== serviceFile)
    report(node, 'repository import outside service')
}
function checkDriverModule(
  node: ts.Node,
  { file, name, report }: ModuleContext,
) {
  if (drivers.test(name) && !file.startsWith(repositoryRoot))
    report(node, 'driver/ORM import outside repository')
  if (
    drivers.test(name) &&
    !name.startsWith('drizzle-orm') &&
    file !== infrastructure
  )
    report(node, 'driver import outside exact infrastructure file')
}
export function checkModule(node: ts.Node, context: ModuleContext) {
  const { name, report } = context
  checkRepositoryModule(node, context)
  checkDriverModule(node, context)
  if (name === 'bun' || name === 'node:module' || name === 'module')
    report(node, 'alternate driver/module loader bypass')
  if (/\.(?:sqlite|sqlite3|db)$/.test(name))
    report(node, 'database file import bypass')
  if (
    name.startsWith('drizzle-orm/sqlite-core') &&
    context.file !== `${repositoryRoot}schema.ts`
  )
    report(node, 'table definitions belong only to shared schema mapping')
  if (name.startsWith('drizzle-orm')) checkOrmModule(node, report)
}
function checkOrmModule(node: ts.Node, report: Reporter) {
  const declaration = node.parent
  checkOrmNamespace(node, report)
  if (ts.isExportDeclaration(declaration) && !declaration.exportClause)
    report(node, 'raw ORM wildcard reexport')
  const visit = (child: ts.Node) => {
    if (ts.isIdentifier(child) && rawOrmNames.has(child.text))
      report(child, 'raw ORM SQL import/type/reexport')
    ts.forEachChild(child, visit)
  }
  visit(declaration)
}
function checkOrmNamespace(node: ts.Node, report: Reporter) {
  const declaration = node.parent
  if (
    ts.isImportDeclaration(declaration) &&
    declaration.importClause?.namedBindings &&
    ts.isNamespaceImport(declaration.importClause.namedBindings)
  )
    report(node, 'raw ORM namespace import')
}
