import type {
  CallNode,
  Expr,
  ExpressionArgument,
  FunctionNode,
  ModuleNode,
  Statement,
  Value,
} from '../services/openscadCompiler'
import {
  compactDiagnosticText,
  evaluationError,
  isFunctionValue,
} from '../services/openScadStableExpressionEval'
import type {
  OpenScad2021LegacyImportFormat,
  OpenScadImportForcedAsset,
} from '../services/openScadImport'
import type { OpenScadProject } from '../services/openScadProject'
import { formatOpenScadValue, isOpenScadRange } from '../services/openScadValueSemantics'
import type { EvalContext } from './types'

export function valueToString(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(valueToString).join(', ')}]`
  if (isFunctionValue(value)) return 'function(...)'
  if (value === undefined) return 'undef'
  return String(value)
}

export function identityValue(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(identityValue).join(',')}]`
  if (isFunctionValue(value)) return `function:${value.name ?? '<anonymous>'}`
  if (isOpenScadRange(value)) return `range:${formatOpenScadValue(value)}`
  if (value === undefined) return 'undef'
  if (typeof value === 'string') return JSON.stringify(value)
  return String(value)
}

/** OpenSCAD boolean conversion: false, undef, zero and empty containers are false. */
export function truthy(value: Value): boolean {
  return value !== false
    && value !== undefined
    && value !== 0
    && value !== ''
    && (!Array.isArray(value) || value.length > 0)
}

export function deepEqual(a: Value, b: Value): boolean {
  if (Array.isArray(a) && Array.isArray(b)) return a.length === b.length && a.every((value, index) => deepEqual(value, b[index]))
  return a === b
}

export function finiteNumber(value: Value, ctx: EvalContext, p: number, label: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) evaluationError(ctx, p, `${label} must be a finite number`)
  return value
}

export function vectorValue(value: Value, ctx: EvalContext, p: number, label: string): number[] {
  if (!Array.isArray(value)) evaluationError(ctx, p, `${label} must be a vector`)
  return value.map(item => finiteNumber(item, ctx, p, label))
}

export function callExpressionArguments(node: CallNode): ExpressionArgument[] {
  if (node.callArguments !== undefined) return [...node.callArguments]
  return Object.entries(node.args).map(([key, value]) => {
    const span = node.argSpans[key]
    return {
      name: node.argKinds[key] === 'named' ? key : undefined,
      value,
      p: span?.start ?? value.p,
      end: span?.end ?? value.p,
    }
  })
}

export interface BoundAssertArguments {
  condition: Expr
  conditionText: string
  message?: Expr
}

export function bindAssertArguments(node: CallNode, ctx: EvalContext): BoundAssertArguments {
  const keys = Object.keys(node.args)
  const unknown = keys.find(key => node.argKinds[key] === 'named'
    ? key !== 'condition' && key !== 'message'
    : key !== '_0' && key !== '_1')
  if (unknown) evaluationError(ctx, node.p, `assert() does not accept argument ${unknown}`)

  const has = (key: string) => Object.prototype.hasOwnProperty.call(node.args, key)
  if (has('_0') && has('condition')) {
    evaluationError(ctx, node.p, 'assert() condition was provided more than once')
  }
  if (has('_1') && has('message')) {
    evaluationError(ctx, node.p, 'assert() message was provided more than once')
  }

  const conditionKey = has('condition') ? 'condition' : has('_0') ? '_0' : null
  if (!conditionKey) evaluationError(ctx, node.p, 'assert() requires a condition')
  const messageKey = has('message') ? 'message' : has('_1') ? '_1' : null
  const span = node.argSpans[conditionKey]
  const rawCondition = span ? ctx.source.slice(span.start, span.end) : 'condition'

  return {
    condition: node.args[conditionKey],
    conditionText: compactDiagnosticText(rawCondition),
    message: messageKey ? node.args[messageKey] : undefined,
  }
}

export function collectModules(nodes: readonly Statement[], modules: Map<string, ModuleNode>): void {
  for (const node of nodes) {
    if (node.type === 'module') modules.set(node.name, node)
    if (node.type === 'call') {
      collectModules(node.children, modules)
      collectModules(node.alternative, modules)
    }
  }
}

export function collectFunctions(nodes: readonly Statement[], functions: Map<string, FunctionNode>): void {
  for (const node of nodes) {
    if (node.type === 'function') functions.set(node.name, node)
    if (node.type === 'call') {
      collectFunctions(node.children, functions)
      collectFunctions(node.alternative, functions)
    } else if (node.type === 'module') collectFunctions(node.children, functions)
  }
}

export const STABLE_COMPATIBILITY_MODULE_FORMAT: Readonly<Partial<Record<string, OpenScad2021LegacyImportFormat>>> = Object.freeze({
  import_stl: 'stl',
  import_off: 'off',
  import_dxf: 'dxf',
  dxf_linear_extrude: 'dxf',
  dxf_rotate_extrude: 'dxf',
})

/**
 * Discover which format-pinned 2021 loaders may execute, then prepare that
 * decoder for every bounded VFS file. The all-files expansion is deliberate:
 * OpenSCAD file expressions can be fully dynamic, so literal-only discovery
 * would make `import_stl(file = condition ? a : b)` depend on the extension.
 */
export function stableCompatibilityForcedAssets(
  project: OpenScadProject,
  nodes: readonly Statement[],
): readonly OpenScadImportForcedAsset[] {
  const required = new Set<OpenScad2021LegacyImportFormat>()
  const visitArguments = (args: readonly ExpressionArgument[]): void => {
    for (const argument of args) visitExpression(argument.value)
  }
  const visitExpression = (expression: Expr): void => {
    switch (expression.kind) {
      case 'literal':
      case 'identifier':
        return
      case 'vector':
        expression.items.forEach(visitExpression)
        return
      case 'range':
        visitExpression(expression.start)
        if (expression.step !== undefined) visitExpression(expression.step)
        visitExpression(expression.end)
        return
      case 'unary':
        visitExpression(expression.value)
        return
      case 'binary':
        visitExpression(expression.left)
        visitExpression(expression.right)
        return
      case 'ternary':
        visitExpression(expression.test)
        visitExpression(expression.yes)
        visitExpression(expression.no)
        return
      case 'function':
        for (const parameter of expression.params) {
          if (parameter.defaultValue !== undefined) visitExpression(parameter.defaultValue)
        }
        visitExpression(expression.body)
        return
      case 'call':
        if (expression.name === 'dxf_dim' || expression.name === 'dxf_cross') required.add('dxf')
        visitExpression(expression.callee)
        visitArguments(expression.args)
        return
      case 'index':
        visitExpression(expression.value)
        visitExpression(expression.index)
        return
      case 'member':
        visitExpression(expression.value)
        return
      case 'let':
        visitArguments(expression.args)
        visitExpression(expression.body)
        return
      case 'assert':
      case 'echo':
        visitArguments(expression.args)
        if (expression.body !== undefined) visitExpression(expression.body)
        return
      case 'lc-for':
        visitArguments(expression.args)
        visitExpression(expression.body)
        return
      case 'lc-for-c':
        visitArguments(expression.init)
        visitExpression(expression.condition)
        visitArguments(expression.update)
        visitExpression(expression.body)
        return
      case 'lc-if':
        visitExpression(expression.condition)
        visitExpression(expression.yes)
        if (expression.no !== undefined) visitExpression(expression.no)
        return
      case 'lc-let':
        visitArguments(expression.args)
        visitExpression(expression.body)
        return
      case 'lc-each':
        visitExpression(expression.value)
        return
    }
  }
  const visitStatements = (statements: readonly Statement[]): void => {
    for (const statement of statements) {
      if (statement.type === 'assign') {
        visitExpression(statement.value)
        continue
      }
      if (statement.type === 'function') {
        for (const parameter of statement.params) {
          if (parameter.defaultValue !== undefined) visitExpression(parameter.defaultValue)
        }
        visitExpression(statement.body)
        continue
      }
      if (statement.type === 'module') {
        for (const parameter of statement.params) {
          if (parameter.defaultValue !== undefined) visitExpression(parameter.defaultValue)
        }
        visitStatements(statement.children)
        continue
      }
      const forced = STABLE_COMPATIBILITY_MODULE_FORMAT[statement.name]
      if (forced !== undefined) required.add(forced)
      visitArguments(callExpressionArguments(statement))
      visitStatements(statement.children)
      visitStatements(statement.alternative)
    }
  }
  visitStatements(nodes)

  const formats = (['stl', 'off', 'dxf'] as const).filter(format => required.has(format))
  return Object.freeze(formats.flatMap(format => (
    project.list().map(file => Object.freeze({ path: file.path, format }))
  )))
}
