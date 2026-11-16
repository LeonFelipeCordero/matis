import ts from 'typescript-ast'

function typeDeclarations(type: ts.Type) {
  return [
    ...(type.getSymbol()?.declarations ?? []),
    ...(type.aliasSymbol?.declarations ?? []),
  ]
}
function persistenceDeclaration(node: ts.Node) {
  return (
    (ts.isClassDeclaration(node) ||
      ts.isInterfaceDeclaration(node) ||
      ts.isTypeAliasDeclaration(node) ||
      ts.isFunctionDeclaration(node)) &&
    /persistence\/repositories\/|node_modules\/(?:drizzle-orm|bun-types\/sqlite)/.test(
      node.getSourceFile().fileName,
    )
  )
}
function persistenceType(type: ts.Type) {
  const declarations = typeDeclarations(type)
  return (
    declarations.some(
      (node) =>
        ts.isTypeAliasDeclaration(node) &&
        /repository-ports\.ts$/.test(node.getSourceFile().fileName),
    ) || declarations.some(persistenceDeclaration)
  )
}
function applicationDeclaration(node: ts.Node) {
  return node.getSourceFile().fileName.includes('/src/')
}
function applicationMembers(type: ts.Type) {
  if (!typeDeclarations(type).some(applicationDeclaration)) return []
  return type
    .getProperties()
    .filter((member) => member.declarations?.some(applicationDeclaration))
}
function typeArguments(type: ts.Type, checker: ts.TypeChecker) {
  const references =
    type.flags & ts.TypeFlags.Object &&
    (type as ts.ObjectType).objectFlags & ts.ObjectFlags.Reference
      ? checker.getTypeArguments(type as ts.TypeReference)
      : []
  return [...(type.aliasTypeArguments ?? []), ...references].filter(
    (argument) =>
      type.aliasSymbol?.name !== 'Record' ||
      !(argument.flags & ts.TypeFlags.Unknown),
  )
}
type ExportContext = {
  checker: ts.TypeChecker
  source: ts.SourceFile
  errors: Array<string>
}
function exportInspector({ checker, source, errors }: ExportContext) {
  const seen = new Set<ts.Type>()
  const inspect = (type: ts.Type, name: string) => {
    if (seen.has(type)) return
    seen.add(type)
    if (type.flags & (ts.TypeFlags.Any | ts.TypeFlags.Unknown)) {
      errors.push(`${name}: opaque service export`)
      return
    }
    if (persistenceType(type)) {
      errors.push(`${name}: persistence handle escapes service`)
      return
    }
    if (type.isUnionOrIntersection())
      type.types.forEach((part) => inspect(part, name))
    // Built-in Error/Array/Map members aren't application persistence handles.
    applicationMembers(type).forEach((member) =>
      inspect(checker.getTypeOfSymbolAtLocation(member, source), name),
    )
    inspectSignatures(type, {
      checker,
      source,
      inspect: (part) => inspect(part, name),
    })
    typeArguments(type, checker).forEach((argument) => inspect(argument, name))
  }
  return (type: ts.Type, name: string) => {
    seen.clear()
    inspect(type, name)
  }
}
function inspectSignatures(
  type: ts.Type,
  context: {
    checker: ts.TypeChecker
    source: ts.SourceFile
    inspect: (type: ts.Type) => void
  },
) {
  const { checker, source, inspect } = context
  const signatures = [
    ...type.getCallSignatures(),
    ...type.getConstructSignatures(),
  ]
  signatures.forEach((signature) => {
    inspect(signature.getReturnType())
    signature.parameters.forEach((parameter) => {
      const argument = checker.getTypeOfSymbolAtLocation(parameter, source)
      if (!(argument.flags & ts.TypeFlags.Unknown)) inspect(argument)
    })
  })
}
/** Follow inferred/public types too: a service cannot reexport or return a repository/ORM handle. */
export function auditServiceExports(
  program: ts.Program,
  source: ts.SourceFile,
): Array<string> {
  const checker = program.getTypeChecker()
  const symbol = checker.getSymbolAtLocation(source)
  if (!symbol) return []
  const errors: Array<string> = []
  const inspect = exportInspector({ checker, source, errors })
  checker.getExportsOfModule(symbol).forEach((exported) => {
    inspect(
      exported.flags & (ts.SymbolFlags.TypeAlias | ts.SymbolFlags.Interface)
        ? checker.getDeclaredTypeOfSymbol(exported)
        : checker.getTypeOfSymbolAtLocation(exported, source),
      exported.name,
    )
  })
  return errors
}
