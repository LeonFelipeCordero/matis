import ts from 'typescript-ast'
import {
  factory,
  infrastructure,
  ormMethods,
  rawMethods,
  rawOrmMembers,
  rawOrmNames,
  repositoryRoot,
  serviceFile,
} from './database-policy'
import {
  accessOwner,
  allowedPragma,
  literal,
  property,
  rootName,
} from './database-syntax'
import { repositoryPort } from './database-ownership-policy'
import type { Reporter } from './database-syntax'

type AccessContext = { file: string; sensitive: Set<string>; report: Reporter }
function pragmaCall(node: ts.Node, file: string) {
  return (
    property(node) === 'run' &&
    ts.isCallExpression(node.parent) &&
    node.parent.arguments.length === 1 &&
    allowedPragma(node.parent.arguments[0], file)
  )
}
function sensitiveOwner(node: ts.Node, sensitive: Set<string>) {
  const owner = accessOwner(node)
  return owner !== undefined && sensitive.has(rootName(owner) ?? '')
}
function checkRawAccess(node: ts.Node, { file, report }: AccessContext) {
  const member = property(node)
  if (member && rawMethods.has(member) && !pragmaCall(node, file))
    report(node, 'raw driver/query access')
  if (member === 'SQL') report(node, 'Bun SQL driver bypass')
  if (member && (rawOrmMembers.has(member) || rawOrmNames.has(member)))
    report(node, 'raw ORM query chunk/wrapper access')
}
function checkOrmAccess(
  node: ts.Node,
  { file, sensitive, report }: AccessContext,
) {
  const member = property(node)
  if (!member) return
  if (
    ormMethods.has(member) &&
    !repositoryPort(file) &&
    !(file === `${factory}.ts` && member === 'transaction')
  )
    report(
      node,
      'ORM/driver call outside exact table repository or snapshot composition',
    )
  if (['update', 'delete'].includes(member))
    checkWriteAccess(node, { file, sensitive, report })
  if (
    file === `${factory}.ts` &&
    ormMethods.has(member) &&
    member !== 'transaction'
  )
    report(node, 'composition cannot query tables')
  if (
    member === 'transaction' &&
    file.startsWith(repositoryRoot) &&
    file !== `${factory}.ts`
  )
    report(node, 'only composition owns the read snapshot')
  if (file === infrastructure && ormMethods.has(member))
    report(node, 'connection infrastructure cannot query domain data')
}
function checkWriteAccess(
  node: ts.Node,
  { file, sensitive, report }: AccessContext,
) {
  if (file === infrastructure)
    report(node, 'connection infrastructure cannot query domain data')
  if (sensitiveOwner(node, sensitive) && !file.startsWith(repositoryRoot))
    report(node, 'ORM write outside repository')
}
function checkOperationAccess(
  node: ts.Node,
  { file, sensitive, report }: AccessContext,
) {
  if (file.startsWith(repositoryRoot) || file === serviceFile) return
  if (!ts.isPropertyAccessExpression(node)) return
  if (
    ['register', 'list', 'view'].includes(node.name.text) &&
    sensitive.has(rootName(node.expression) ?? '')
  )
    report(node, 'repository operation outside service')
}
function checkComputedBinding(
  node: ts.Node,
  { sensitive, report }: AccessContext,
) {
  if (!ts.isBindingElement(node) || !node.propertyName) return
  if (
    ts.isComputedPropertyName(node.propertyName) &&
    sensitiveOwner(node, sensitive)
  )
    report(node, 'computed persistence destructuring bypass')
}
function invokedAccess(node: ts.Node) {
  return (
    (ts.isCallExpression(node.parent) || ts.isNewExpression(node.parent)) &&
    node.parent.expression === node
  )
}
function checkComputedAccess(
  node: ts.Node,
  { sensitive, report }: AccessContext,
) {
  if (
    !ts.isElementAccessExpression(node) ||
    literal(node.argumentExpression) !== undefined
  )
    return
  if (sensitive.has(rootName(node.expression) ?? ''))
    report(node, 'computed persistence/reflection binding bypass')
  if (invokedAccess(node)) report(node, 'computed method/constructor bypass')
}
export function checkAccess(node: ts.Node, context: AccessContext) {
  checkRawAccess(node, context)
  checkOrmAccess(node, context)
  checkOperationAccess(node, context)
  checkComputedBinding(node, context)
  checkComputedAccess(node, context)
  if (ts.isIdentifier(node) && rawOrmNames.has(node.text))
    context.report(node, 'raw ORM SQL constructor/type')
}
