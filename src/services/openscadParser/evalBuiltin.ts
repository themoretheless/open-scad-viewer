import { CooperativeCheckpoint, MAX_SHAPES, STABLE_COMPATIBILITY_MODULE_FORMAT, StableBuiltinValueError, StableViewportRootSelection, YIELD_EVERY_STATEMENTS, enterStableStatementScope } from '../openscadParser'
import type { BoundAssertArguments, EvalContext, Shape } from '../openscadParser'
import { hasOpenScadViewportModifier } from '../openscadCompiler'
import type { CallNode, Expr, ExpressionArgument, FunctionNode, ModuleNode, Statement, Value } from '../openscadCompiler'
import { MAX_EVAL_OPS, MAX_VALUE_ELEMENTS, compactDiagnosticText, evaluationError, isFunctionValue, isStableProfile, registerArrayValue } from '../openScadStableExpressionEval'
import type { OpenScad2021LegacyImportFormat, OpenScadImportForcedAsset } from '../openScadImport'
import { createDeferredOpenScadBuiltinArguments, evaluateOpenScadBuiltinFunction } from '../openScadBuiltinFunctions'
import type { OpenScadBuiltinValue } from '../openScadBuiltinFunctions'
import type { OpenScadProject } from '../openScadProject'
import { evalDxfQueryBuiltin, evalExpression, registerStringValue, resolveStableVariable, warn } from './isExclusiveView'
import { evalNode } from './evalNode'

export function evalBuiltin(expr: Extract<Expr, { kind: 'call' }>, ctx: EvalContext, depth: number): Value {
  const name = expr.name
  if (name === null) evaluationError(ctx, expr.p, 'Expression is not callable')
  if (name === 'assert') {
    evaluationError(ctx, expr.p, 'Expression-form assert() is not supported; use statement assert()')
  }
  if (isStableProfile(ctx) && (name === 'dxf_dim' || name === 'dxf_cross')) {
    return evalDxfQueryBuiltin(expr, ctx, depth)
  }
  if (!isStableProfile(ctx) && expr.args.some(argument => argument.name !== undefined)) {
    evaluationError(ctx, expr.p, `${name}() does not accept named arguments in this engine revision`)
  }
  const evaluateArgument = (arg: ExpressionArgument): Value => {
    // OpenSCAD deliberately permits probing an undeclared bare name with
    // is_undef() without emitting the ordinary unknown-variable warning.
    if (isStableProfile(ctx) && name === 'is_undef' && expr.args.length === 1
      && arg.value.kind === 'identifier') {
      if (arg.value.name === 'PI') return Math.PI
      const resolved = resolveStableVariable(arg.value.name, ctx)
      return resolved.found ? resolved.value : undefined
    }
    return evalExpression(arg.value, ctx, depth + 1)
  }
  const values = isStableProfile(ctx)
    ? createDeferredOpenScadBuiltinArguments(
        expr.args.length,
        index => evaluateArgument(expr.args[index]),
      )
    : expr.args.map(evaluateArgument)
  let result: ReturnType<typeof evaluateOpenScadBuiltinFunction>
  try {
    result = evaluateOpenScadBuiltinFunction(
      name,
      values as OpenScadBuiltinValue[],
      {
        error: message => {
          if (isStableProfile(ctx)) throw new StableBuiltinValueError(message)
          return evaluationError(ctx, expr.p, message)
        },
        warning: message => warn(ctx, message),
        registerArray: (items, label) => {
          if (items.length > MAX_VALUE_ELEMENTS) {
            evaluationError(ctx, expr.p, `${label} exceeds ${MAX_VALUE_ELEMENTS.toLocaleString()} elements`)
          }
          return registerArrayValue([...items] as Value[], ctx, expr.p, label)
        },
        registerString: (value, label) => {
          if (value.length > MAX_VALUE_ELEMENTS) {
            evaluationError(ctx, expr.p, `${label} exceeds ${MAX_VALUE_ELEMENTS.toLocaleString()} characters`)
          }
          return registerStringValue(value, ctx, expr.p, label)
        },
        random: Math.random,
        parentModule: moduleDepth => ctx.moduleStack.at(-1 - moduleDepth),
        isFunction: value => isFunctionValue(value as Value),
      },
    )
  } catch (error) {
    if (!(error instanceof StableBuiltinValueError)) throw error
    warn(ctx, error.message)
    return undefined
  }
  if (!result.recognized) evaluationError(ctx, expr.p, `Unsupported function ${name}()`)
  return result.value as Value
}

export function valueToString(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(valueToString).join(', ')}]`
  if (isFunctionValue(value)) return 'function(...)'
  if (value === undefined) return 'undef'
  return String(value)
}

/** OpenSCAD boolean conversion: false, undef, zero and empty containers are false. */
export function truthy(value: Value) {
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

export function arg(node: CallNode, name: string, position: number, fallback: Value, ctx: EvalContext): Value {
  const expression = node.args[name] ?? node.args[`_${position}`]
  return expression ? evalExpression(expression, ctx) : fallback
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

export function collectModules(nodes: readonly Statement[], modules: Map<string, ModuleNode>) {
  for (const node of nodes) {
    if (node.type === 'module') modules.set(node.name, node)
    if (node.type === 'call') {
      collectModules(node.children, modules)
      collectModules(node.alternative, modules)
    }
  }
}

export function collectFunctions(nodes: readonly Statement[], functions: Map<string, FunctionNode>) {
  for (const node of nodes) {
    if (node.type === 'function') functions.set(node.name, node)
    if (node.type === 'call') {
      collectFunctions(node.children, functions)
      collectFunctions(node.alternative, functions)
    } else if (node.type === 'module') collectFunctions(node.children, functions)
  }
}

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

export async function evalNodes(nodes: readonly Statement[], parent: EvalContext, scoped = true): Promise<Shape[]> {
  const ctx: EvalContext = isStableProfile(parent)
    ? enterStableStatementScope(nodes, parent)
    : { ...parent, env: scoped ? new Map(parent.env) : parent.env }
  return await evalPreparedNodes(nodes, ctx)
}

export function stableViewportDisabled(statement: Statement, ctx: EvalContext): boolean {
  return isStableProfile(ctx)
    && statement.type === 'call'
    && hasOpenScadViewportModifier(statement, 'disable')
}

/**
 * GeometryEvaluationResult has one undifferentiated mesh collection, so it
 * cannot safely expose OpenSCAD's background preview layer as ordinary solid
 * geometry. Evaluate `%` for language effects in both qualities, omit its
 * shapes, and make the preview limitation explicit instead of claiming a
 * background layer that downstream consumers cannot distinguish.
 */
export async function evalPreparedCall(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  const background = isStableProfile(ctx)
    && ctx.viewportRootOwner !== node
    && hasOpenScadViewportModifier(node, 'background')
  const shapes = await evalNode(node, ctx)
  if (!background) return shapes
  if (ctx.quality === 'preview' && shapes.length > 0) {
    warn(ctx, 'Viewport background (%) geometry is omitted in preview because the mesh result contract has no background-layer metadata')
  }
  return []
}

export async function evalPreparedNodes(nodes: readonly Statement[], ctx: EvalContext): Promise<Shape[]> {
  const output: Shape[] = []
  let statementsSinceYield = 0
  for (const node of nodes) {
    if (ctx.control && statementsSinceYield >= YIELD_EVERY_STATEMENTS) {
      statementsSinceYield = 0
      await ctx.control.yieldIfDue()
    }
    statementsSinceYield++
    // Unlike every other viewport modifier, `*` suppresses argument, effect,
    // child, and geometry evaluation for the complete call subtree.
    if (stableViewportDisabled(node, ctx)) continue
    if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
    if (node.type === 'assign') {
      if (!isStableProfile(ctx)) ctx.env.set(node.name, evalExpression(node.value, ctx))
      continue
    }
    if (node.type === 'module' || node.type === 'function') continue
    output.push(...await evalPreparedCall(node, ctx))
    if (output.length > MAX_SHAPES) evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
  }
  return output
}

/**
 * Top-level evaluation with cooperative cancellation. Nested `for` / `if` /
 * module bodies share the same checkpoint, so a Worker can receive cancel
 * mid-loop. A single enormous kernel/BVH call still cannot yield mid-WASM;
 * the coordinator grace timer remains that hard boundary.
 */
export async function evalTopLevel(nodes: readonly Statement[], ctx: EvalContext, control: CooperativeCheckpoint): Promise<Shape[]> {
  control.poll()
  try {
    return await evalPreparedNodes(nodes, { ...ctx, control })
  } catch (error) {
    if (error instanceof StableViewportRootSelection) return error.shapes
    throw error
  }
}

export function viewportRootActivates(node: CallNode, shapes: readonly Shape[]): boolean {
  if (shapes.length > 0) return true
  // OpenSCAD does not create an empty CSG container for these transparent
  // effect/branch calls. Every other evaluated module instantiation can win a
  // root selection even when its resulting geometry is empty.
  return !['if', 'assert', 'echo', 'children', 'child'].includes(node.name)
}

export function passedCallChildren(ctx: EvalContext): {
  readonly statements: readonly Statement[]
  readonly context: EvalContext
} {
  const passed = ctx.callChildren
  const statements = Array.isArray(passed) ? passed : passed?.statements ?? []
  if (Array.isArray(passed) || passed === undefined) return { statements, context: ctx }
  return {
    statements,
    context: {
      ...ctx,
      env: new Map(passed.env),
      stableScope: passed.scope,
      scopeVisibleBefore: Number.POSITIVE_INFINITY,
      callChildren: passed.continuation,
    },
  }
}

export function compatibilityDeprecation(
  node: CallNode,
  ctx: EvalContext,
  replacement: string,
): void {
  const message = node.name === 'child'
    ? 'child() will be removed in future releases. Use children() instead.'
    : `The ${node.name}() module will be removed in future releases. Use ${replacement} instead.`
  warn(ctx, message)
}
