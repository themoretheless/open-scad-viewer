import {
  finalizeOpenScadProgram,
  parseOpenScadProjectSource,
  type CallNode,
  type DirectiveNode,
  type Expr,
  type ModuleNode,
  type OpenScadViewportModifier,
  type OpenScadParsedStatement,
  type Statement,
} from './openscadCompiler'
import {
  OpenScadProject,
  resolveOpenScadProjectPath,
} from './openScadProject'

export const OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS = 512
export const OPENSCAD_PROJECT_MAX_EXPANDED_STATEMENTS = 50_000
export const OPENSCAD_PROJECT_MAX_EXPANDED_SYNTAX_NODES = 100_000

export const OPENSCAD_PROJECT_COMPILE_ERROR_CODES = Object.freeze([
  'E_PROJECT_DEPENDENCY_MISSING',
  'E_PROJECT_DEPENDENCY_NOT_SOURCE',
  'E_PROJECT_DEPENDENCY_CYCLE',
  'E_PROJECT_EXPANSION_LIMIT',
  'E_PROJECT_MODIFIER_TARGET',
] as const)

export type OpenScadProjectCompileErrorCode =
  typeof OPENSCAD_PROJECT_COMPILE_ERROR_CODES[number]

export interface OpenScadProjectCompileErrorDetails {
  readonly directive?: DirectiveNode['directive']
  readonly importer?: string
  readonly specifier?: string
  readonly path?: string
  readonly chain?: readonly string[]
  readonly metric?: 'dependency-expansions' | 'statements' | 'syntax-nodes'
  readonly limit?: number
  readonly actual?: number
  readonly modifier?: OpenScadViewportModifier['kind']
  readonly modifierSpan?: OpenScadViewportModifier['span']
}

export class OpenScadProjectCompileError extends Error {
  readonly details: Readonly<OpenScadProjectCompileErrorDetails>

  constructor(
    readonly code: OpenScadProjectCompileErrorCode,
    message: string,
    details: OpenScadProjectCompileErrorDetails = {},
  ) {
    super(message)
    this.name = 'OpenScadProjectCompileError'
    this.details = Object.freeze({
      ...details,
      ...(details.chain === undefined ? {} : { chain: Object.freeze([...details.chain]) }),
    })
  }
}

function isDirective(statement: OpenScadParsedStatement): statement is DirectiveNode {
  return statement.type === 'directive'
}

function countExpressionNodes(expression: Expr): number {
  const countArguments = (args: readonly { readonly value: Expr }[]): number => args.reduce(
    (total, argument) => total + countExpressionNodes(argument.value),
    0,
  )
  switch (expression.kind) {
    case 'literal':
    case 'identifier':
      return 1
    case 'vector':
      return 1 + expression.items.reduce((total, item) => total + countExpressionNodes(item), 0)
    case 'range':
      return 1 + countExpressionNodes(expression.start)
        + (expression.step === undefined ? 0 : countExpressionNodes(expression.step))
        + countExpressionNodes(expression.end)
    case 'unary':
      return 1 + countExpressionNodes(expression.value)
    case 'binary':
      return 1 + countExpressionNodes(expression.left) + countExpressionNodes(expression.right)
    case 'ternary':
      return 1 + countExpressionNodes(expression.test)
        + countExpressionNodes(expression.yes)
        + countExpressionNodes(expression.no)
    case 'function':
      return 1 + expression.params.reduce(
        (total, param) => total + (param.defaultValue === undefined
          ? 0
          : countExpressionNodes(param.defaultValue)),
        0,
      ) + countExpressionNodes(expression.body)
    case 'call':
      return 1 + countExpressionNodes(expression.callee)
        + expression.args.reduce((total, arg) => total + countExpressionNodes(arg.value), 0)
    case 'index':
      return 1 + countExpressionNodes(expression.value) + countExpressionNodes(expression.index)
    case 'member':
      return 1 + countExpressionNodes(expression.value)
    case 'let':
      return 1 + countArguments(expression.args) + countExpressionNodes(expression.body)
    case 'assert':
    case 'echo':
      return 1 + countArguments(expression.args)
        + (expression.body === undefined ? 0 : countExpressionNodes(expression.body))
    case 'lc-for':
      return 1 + countArguments(expression.args) + countExpressionNodes(expression.body)
    case 'lc-for-c':
      return 1 + countArguments(expression.init)
        + countExpressionNodes(expression.condition)
        + countArguments(expression.update)
        + countExpressionNodes(expression.body)
    case 'lc-if':
      return 1 + countExpressionNodes(expression.condition)
        + countExpressionNodes(expression.yes)
        + (expression.no === undefined ? 0 : countExpressionNodes(expression.no))
    case 'lc-let':
      return 1 + countArguments(expression.args) + countExpressionNodes(expression.body)
    case 'lc-each':
      return 1 + countExpressionNodes(expression.value)
  }
}

function countSyntaxNodes(statements: readonly OpenScadParsedStatement[]): number {
  let total = 0
  for (const statement of statements) {
    total++
    if (statement.type === 'directive') continue
    if (statement.type === 'assign') {
      total += countExpressionNodes(statement.value)
    } else if (statement.type === 'function') {
      total += statement.params.reduce(
        (sum, param) => sum + (param.defaultValue === undefined
          ? 0
          : countExpressionNodes(param.defaultValue)),
        0,
      ) + countExpressionNodes(statement.body)
    } else {
      total += statement.type === 'call'
        ? (statement.callArguments ?? Object.values(statement.args).map(value => ({ value }))).reduce(
          (sum, argument) => sum + countExpressionNodes(argument.value),
          0,
        )
        : statement.params.reduce(
          (sum, param) => sum + (param.defaultValue === undefined
            ? 0
            : countExpressionNodes(param.defaultValue)),
          0,
        )
      total += countSyntaxNodes(statement.children as readonly OpenScadParsedStatement[])
      if (statement.type === 'call') {
        total += countSyntaxNodes(statement.alternative as readonly OpenScadParsedStatement[])
      }
    }
  }
  return total
}

/**
 * Expands one immutable project snapshot into the executable independent-engine
 * AST. Includes retain statement position; uses retain declarations only.
 */
export function compileOpenScadProject(project: OpenScadProject): readonly Statement[] {
  const parsedFiles = new Map<string, readonly OpenScadParsedStatement[]>()
  const parsedFileWeights = new Map<string, number>()
  let dependencyExpansions = 0
  let expandedStatements = 0
  let expandedSyntaxNodes = 0

  interface PendingViewportModifier {
    readonly modifier: OpenScadViewportModifier
    readonly sourcePath: string
  }
  interface ExpandedStatements {
    readonly statements: Statement[]
    readonly pending: PendingViewportModifier[]
  }

  const reserveStatement = (path: string): void => {
    expandedStatements++
    if (expandedStatements > OPENSCAD_PROJECT_MAX_EXPANDED_STATEMENTS) {
      throw new OpenScadProjectCompileError(
        'E_PROJECT_EXPANSION_LIMIT',
        `OpenSCAD project expansion exceeds ${OPENSCAD_PROJECT_MAX_EXPANDED_STATEMENTS.toLocaleString()} statements.`,
        {
          path,
          metric: 'statements',
          limit: OPENSCAD_PROJECT_MAX_EXPANDED_STATEMENTS,
          actual: expandedStatements,
        },
      )
    }
  }

  const reserveDependency = (
    importer: string,
    directive: DirectiveNode,
    path: string,
  ): void => {
    dependencyExpansions++
    if (dependencyExpansions > OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS) {
      throw new OpenScadProjectCompileError(
        'E_PROJECT_EXPANSION_LIMIT',
        `OpenSCAD project expansion exceeds ${OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS.toLocaleString()} dependency directives.`,
        {
          directive: directive.directive,
          importer,
          specifier: directive.path,
          path,
          metric: 'dependency-expansions',
          limit: OPENSCAD_PROJECT_MAX_DEPENDENCY_EXPANSIONS,
          actual: dependencyExpansions,
        },
      )
    }
  }

  const parseFile = (path: string): readonly OpenScadParsedStatement[] => {
    const cached = parsedFiles.get(path)
    if (cached !== undefined) return cached
    const file = project.read(path)
    // The entrypoint and every dependency are checked before reaching here.
    if (file?.kind !== 'source') throw new Error(`Project source ${path} is unavailable`)
    const parsed = parseOpenScadProjectSource(file.source)
    parsedFiles.set(path, parsed)
    parsedFileWeights.set(path, countSyntaxNodes(parsed))
    return parsed
  }

  const reserveSyntaxNodes = (path: string, parsed: readonly OpenScadParsedStatement[]): void => {
    const weight = parsedFileWeights.get(path) ?? countSyntaxNodes(parsed)
    expandedSyntaxNodes += weight
    if (expandedSyntaxNodes > OPENSCAD_PROJECT_MAX_EXPANDED_SYNTAX_NODES) {
      throw new OpenScadProjectCompileError(
        'E_PROJECT_EXPANSION_LIMIT',
        `OpenSCAD project expansion exceeds ${OPENSCAD_PROJECT_MAX_EXPANDED_SYNTAX_NODES.toLocaleString()} syntax nodes.`,
        {
          path,
          metric: 'syntax-nodes',
          limit: OPENSCAD_PROJECT_MAX_EXPANDED_SYNTAX_NODES,
          actual: expandedSyntaxNodes,
        },
      )
    }
  }

  const expandDependency = (
    directive: DirectiveNode,
    importer: string,
    definitionsOnly: boolean,
    stack: readonly string[],
    leadingModifiers: readonly PendingViewportModifier[],
  ): ExpandedStatements => {
    const path = resolveOpenScadProjectPath(importer, directive.path)
    const dependency = project.read(path)
    if (dependency === null) {
      throw new OpenScadProjectCompileError(
        'E_PROJECT_DEPENDENCY_MISSING',
        `${directive.directive} from ${importer} refers to missing project file ${path}.`,
        {
          directive: directive.directive,
          importer,
          specifier: directive.path,
          path,
        },
      )
    }
    if (dependency.kind !== 'source') {
      throw new OpenScadProjectCompileError(
        'E_PROJECT_DEPENDENCY_NOT_SOURCE',
        `${directive.directive} from ${importer} cannot load non-source project file ${path}.`,
        {
          directive: directive.directive,
          importer,
          specifier: directive.path,
          path,
        },
      )
    }

    const cycleStart = stack.indexOf(path)
    if (cycleStart >= 0) {
      const chain = [...stack.slice(cycleStart), path]
      throw new OpenScadProjectCompileError(
        'E_PROJECT_DEPENDENCY_CYCLE',
        `OpenSCAD project dependency cycle: ${chain.join(' -> ')}.`,
        {
          directive: directive.directive,
          importer,
          specifier: directive.path,
          path,
          chain,
        },
      )
    }

    reserveDependency(importer, directive, path)
    return expandFile(
      path,
      definitionsOnly || directive.directive === 'use',
      [...stack, path],
      leadingModifiers,
    )
  }

  const rejectModifierTarget = (
    pending: readonly PendingViewportModifier[],
    targetPath: string,
    target: Statement | 'end-of-block' | 'end-of-project',
  ): never => {
    const first = pending[0]
    const targetName = typeof target === 'string'
      ? target
      : `${target.type} ${target.name}`
    throw new OpenScadProjectCompileError(
      'E_PROJECT_MODIFIER_TARGET',
      `Viewport modifier ${first.modifier.token} from ${first.sourcePath} cannot prefix ${targetName} in ${targetPath}.`,
      {
        path: targetPath,
        modifier: first.modifier.kind,
        modifierSpan: first.modifier.span,
      },
    )
  }

  const cloneStatement = (
    statement: Statement,
    path: string,
    stack: readonly string[],
  ): Statement => {
    reserveStatement(path)
    if (statement.type === 'call') {
      const {
        children: sourceChildren,
        alternative: sourceAlternative,
        operationId: _operationId,
        ...fields
      } = statement
      return {
        ...structuredClone(fields),
        sourcePath: path,
        children: expandBodyStatements(
          sourceChildren as readonly OpenScadParsedStatement[],
          path,
          stack,
        ),
        alternative: expandBodyStatements(
          sourceAlternative as readonly OpenScadParsedStatement[],
          path,
          stack,
        ),
      } as CallNode
    }
    if (statement.type === 'module') {
      const { children: sourceChildren, ...fields } = statement
      return {
        ...structuredClone(fields),
        children: expandBodyStatements(
          sourceChildren as readonly OpenScadParsedStatement[],
          path,
          stack,
        ),
      } as ModuleNode
    }
    return structuredClone(statement)
  }

  const expandStatements = (
    statements: readonly OpenScadParsedStatement[],
    path: string,
    definitionsOnly: boolean,
    stack: readonly string[],
    leadingModifiers: readonly PendingViewportModifier[] = [],
  ): ExpandedStatements => {
    const expanded: Statement[] = []
    let pending = [...leadingModifiers]
    for (const statement of statements) {
      if (isDirective(statement)) {
        const directiveModifiers = (statement.viewportModifiers ?? []).map(modifier => ({
          modifier,
          sourcePath: path,
        }))
        const dependency = expandDependency(
          statement,
          path,
          definitionsOnly,
          stack,
          [...pending, ...directiveModifiers],
        )
        expanded.push(...dependency.statements)
        pending = dependency.pending
        continue
      }
      const ownModifiers = statement.type === 'call'
        ? (statement.viewportModifiers ?? []).map(modifier => ({ modifier, sourcePath: path }))
        : []
      const effectiveModifiers = [...pending, ...ownModifiers]
      pending = []
      if (effectiveModifiers.length > 0 && statement.type !== 'call') {
        rejectModifierTarget(effectiveModifiers, path, statement)
      }
      if (definitionsOnly && statement.type !== 'module' && statement.type !== 'function') {
        continue
      }
      const cloned = cloneStatement(statement, path, stack)
      expanded.push(effectiveModifiers.length > 0 && cloned.type === 'call'
        ? {
          ...cloned,
          viewportModifiers: effectiveModifiers.map(entry => ({
            ...structuredClone(entry.modifier),
            sourcePath: entry.sourcePath,
          })),
        }
        : cloned)
    }
    return { statements: expanded, pending }
  }

  const expandBodyStatements = (
    statements: readonly OpenScadParsedStatement[],
    path: string,
    stack: readonly string[],
  ): Statement[] => {
    const result = expandStatements(statements, path, false, stack)
    if (result.pending.length > 0) rejectModifierTarget(result.pending, path, 'end-of-block')
    return result.statements
  }

  const expandFile = (
    path: string,
    definitionsOnly: boolean,
    stack: readonly string[],
    leadingModifiers: readonly PendingViewportModifier[] = [],
  ): ExpandedStatements => {
    const parsed = parseFile(path)
    reserveSyntaxNodes(path, parsed)
    return expandStatements(parsed, path, definitionsOnly, stack, leadingModifiers)
  }

  const expanded = expandFile(project.entrypoint, false, [project.entrypoint])
  if (expanded.pending.length > 0) {
    rejectModifierTarget(expanded.pending, project.entrypoint, 'end-of-project')
  }
  return finalizeOpenScadProgram(expanded.statements)
}
