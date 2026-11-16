import ts from 'typescript-ast'
import { factory, repositoryRoot } from './database-policy'
import { permittedPorts, repositoryPort } from './database-ownership-policy'

function applicationDeclaration(node: ts.Node) {
  return /(?:^|\/)src\//.test(node.getSourceFile().fileName)
}

function structuralPort(type: ts.Type, checker: ts.TypeChecker) {
  return ['register', 'list', 'view', 'close'].every((name) => {
    const member = type.getProperty(name)
    const declaration = member?.valueDeclaration ?? member?.declarations?.[0]
    return (
      member &&
      declaration &&
      checker.getTypeOfSymbolAtLocation(member, declaration).getCallSignatures()
        .length > 0
    )
  })
}

function referencedTypes(type: ts.Type, checker: ts.TypeChecker) {
  const references =
    type.flags & ts.TypeFlags.Object &&
    (type as ts.ObjectType).objectFlags & ts.ObjectFlags.Reference
      ? checker.getTypeArguments(type as ts.TypeReference)
      : []
  return [...(type.aliasTypeArguments ?? []), ...references]
}

function dependencyBinding(node: ts.Node) {
  return (
    ts.isParameter(node) ||
    ts.isVariableDeclaration(node) ||
    ts.isPropertyDeclaration(node) ||
    ts.isPropertyAccessExpression(node) ||
    ts.isElementAccessExpression(node)
  )
}

function storageOwner(file: string) {
  return (
    file.includes(repositoryRoot) ||
    /node_modules\/(?:drizzle-orm|bun-types\/sqlite)/.test(file)
  )
}

function rawStorageDeclaration(node: ts.Node) {
  return (
    (ts.isClassDeclaration(node) ||
      ts.isInterfaceDeclaration(node) ||
      (ts.isTypeAliasDeclaration(node) && node.name.text === 'AppDatabase')) &&
    /(?:persistence\/repositories\/connection\.ts|node_modules\/(?:drizzle-orm|bun-types\/sqlite))/.test(
      node.getSourceFile().fileName,
    )
  )
}

function dependencyInspector(
  checker: ts.TypeChecker,
  allowed = new Set<string>(),
  storageAllowed = false,
) {
  const inspect = (type: ts.Type, seen = new Set<ts.Type>()): boolean => {
    if (
      seen.has(type) ||
      type.flags & (ts.TypeFlags.Any | ts.TypeFlags.Unknown)
    )
      return false
    seen.add(type)
    const declarations = [
      ...(type.getSymbol()?.declarations ?? []),
      ...(type.aliasSymbol?.declarations ?? []),
    ]
    if (!storageAllowed && declarations.some(rawStorageDeclaration)) return true
    const port = declarations.find((node) =>
      /(?:^|\/)repository-ports\.ts$/.test(node.getSourceFile().fileName),
    )
    if (port) {
      let declaration: ts.Node = port
      while (
        !ts.isTypeAliasDeclaration(declaration) &&
        !ts.isSourceFile(declaration)
      )
        declaration = declaration.parent
      if (ts.isTypeAliasDeclaration(declaration))
        return !allowed.has(declaration.name.text)
    }
    const implementation = declarations
      .filter(
        (node) => ts.isClassDeclaration(node) || ts.isMethodDeclaration(node),
      )
      .map((node) => repositoryPort(node.getSourceFile().fileName))
      .find(Boolean)
    if (implementation) return !allowed.has(implementation)
    if (type.isUnionOrIntersection())
      return type.types.some((part) => inspect(part, seen))
    // Structural aliases and inferred factory results are ports too, not just named types.
    if (structuralPort(type, checker)) return true
    if (
      referencedTypes(type, checker).some((argument) => inspect(argument, seen))
    )
      return true
    if (
      [...type.getCallSignatures(), ...type.getConstructSignatures()].some(
        (signature) =>
          inspect(signature.getReturnType(), seen) ||
          signature.parameters.some(
            (parameter) =>
              parameter.valueDeclaration &&
              inspect(
                checker.getTypeOfSymbolAtLocation(
                  parameter,
                  parameter.valueDeclaration,
                ),
                seen,
              ),
          ),
      )
    )
      return true
    return type.getProperties().some((member) => {
      const declaration = member.declarations?.find(
        (candidate) =>
          applicationDeclaration(candidate) &&
          !(
            ts.canHaveModifiers(candidate) &&
            ts
              .getModifiers(candidate)
              ?.some(
                (modifier) => modifier.kind === ts.SyntaxKind.PrivateKeyword,
              )
          ),
      )
      return (
        declaration &&
        inspect(checker.getTypeOfSymbolAtLocation(member, declaration), seen)
      )
    })
  }
  return inspect
}

function argumentValue(node: ts.Expression): ts.Expression {
  return ts.isAsExpression(node) ||
    ts.isTypeAssertionExpression(node) ||
    ts.isParenthesizedExpression(node) ||
    ts.isSatisfiesExpression(node)
    ? argumentValue(node.expression)
    : node
}

function dependencyInitializer(symbol: ts.Symbol) {
  const declaration = symbol.valueDeclaration
  return declaration &&
    (ts.isVariableDeclaration(declaration) ||
      ts.isPropertyDeclaration(declaration))
    ? declaration.initializer
    : undefined
}

function dependencyAssignments(source: ts.SourceFile, checker: ts.TypeChecker) {
  const assignments = new Map<ts.Symbol, Array<ts.Expression>>()
  const visit = (node: ts.Node) => {
    if (
      ts.isBinaryExpression(node) &&
      node.operatorToken.kind === ts.SyntaxKind.EqualsToken
    ) {
      const symbol = checker.getSymbolAtLocation(node.left)
      if (symbol)
        assignments.set(symbol, [
          ...(assignments.get(symbol) ?? []),
          node.right,
        ])
    }
    ts.forEachChild(node, visit)
  }
  visit(source)
  return assignments
}

function valueInspector(
  checker: ts.TypeChecker,
  containsDependency: (type: ts.Type) => boolean,
  source: ts.SourceFile,
) {
  const assignments = dependencyAssignments(source, checker)
  const inspect = (
    node: ts.Expression,
    seen = new Set<ts.Symbol>(),
  ): boolean => {
    const value = argumentValue(node)
    if (containsDependency(checker.getTypeAtLocation(value))) return true
    const symbol = checker.getSymbolAtLocation(value)
    if (!symbol || seen.has(symbol)) return false
    seen.add(symbol)
    const initializer = dependencyInitializer(symbol)
    const values = [
      ...(assignments.get(symbol) ?? []),
      ...(initializer ? [initializer] : []),
    ]
    return values.some((candidate) => inspect(candidate, seen))
  }
  return inspect
}

/** Type/dependency check across the program, including private helpers and constructors. */
export function auditRepositoryDependencies(
  program: ts.Program,
  source: ts.SourceFile,
) {
  const checker = program.getTypeChecker()
  const normalized = source.fileName.replaceAll('\\', '/')
  const composition =
    normalized === `${factory}.ts` || normalized.endsWith(`/${factory}.ts`)
  const containsDependency = dependencyInspector(
    checker,
    permittedPorts(normalized),
    storageOwner(normalized),
  )
  const errors: Array<string> = []
  const report = (node: ts.Node, message: string) =>
    errors.push(
      `${source.fileName}:${source.getLineAndCharacterOfPosition(node.getStart()).line + 1}: ${message}`,
    )
  const visit = (node: ts.Node) => {
    if (
      !composition &&
      dependencyBinding(node) &&
      containsDependency(checker.getTypeAtLocation(node))
    )
      report(node, 'repository dependency outside service/repository')
    if (ts.isCallExpression(node) || ts.isNewExpression(node)) {
      const target =
        checker.getResolvedSignature(node)?.declaration?.getSourceFile()
          .fileName ?? ''
      const inspect = valueInspector(
        checker,
        dependencyInspector(
          checker,
          permittedPorts(target),
          storageOwner(target),
        ),
        source,
      )
      if (node.arguments?.some((argument) => inspect(argument)))
        report(
          node,
          'repository dependency delegated outside service/repository',
        )
      if (
        composition &&
        ts.isCallExpression(node) &&
        (ts.isPropertyAccessExpression(node.expression) ||
          ts.isElementAccessExpression(node.expression)) &&
        dependencyInspector(
          checker,
          new Set(),
          true,
        )(checker.getTypeAtLocation(node.expression.expression))
      )
        report(node, 'composition may construct repositories, not call them')
    }
    ts.forEachChild(node, visit)
  }
  visit(source)
  return errors
}
