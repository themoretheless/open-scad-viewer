import type { CadKernelHandle } from '../services/cadKernelOps'
import { boundedSceneEntityId } from '../core/boundedSceneEntityId'
import type { SceneEntityId, SourceOperationId } from '../core/mesh'
import { AbortedError, OpenSCADParseError, positionKernelError } from '../services/openscadErrors'
import {
  createDeferredOpenScadBuiltinArguments,
  evaluateOpenScadBuiltinFunction,
  type OpenScadBuiltinValue,
} from '../services/openScadBuiltinFunctions'
import {
  TT,
  type CallNode,
  type Expr,
  type ExpressionArgument,
  type FunctionValue,
  type Value,
} from '../services/openscadCompiler'
import {
  formatOpenScadValue,
  openScadIndex,
  openScadMember,
  openScadTruthy,
  openScadUnary,
} from '../services/openScadValueSemantics'
import {
  MAX_EVAL_OPS,
  MAX_EVALUATED_VALUE_UNITS,
  MAX_EXPRESSION_DEPTH,
  MAX_RANGE_ITEMS,
  MAX_VALUE_ELEMENTS,
  appendComprehensionValues,
  createStableExpressionEvaluators,
  evaluationError,
  isFunctionValue,
  isListComprehensionExpression,
  isStableProfile,
  registerArrayValue,
  stableOverlayContext,
  type StableFunctionValue,
} from '../services/openScadStableExpressionEval'
import {
  queryPreparedOpenScadDxfCross,
  queryPreparedOpenScadDxfDimension,
} from '../services/openScadImport'
import type { RGBA } from '../services/openscadColors'
import {
  StableBuiltinValueError,
  type EvalContext,
  type Shape3D,
} from './types'
import {
  callExpressionArguments,
  finiteNumber,
  truthy,
} from './astDiscovery'

export function currentEntityId(ctx: EvalContext): SceneEntityId {
  return boundedSceneEntityId(ctx.instancePath)
}

export function staticOperationId(node: CallNode): SourceOperationId {
  return node.operationId ?? `op:legacy-offset-${node.p}`
}

export function trackSource(geometry: CadKernelHandle, node: CallNode, ctx: EvalContext) {
  const originalId = ctx.kernel.originalId(geometry)
  if (originalId === null || originalId < 0 || ctx.sourceReferences.has(originalId)) return
  ctx.sourceReferences.set(originalId, {
    id: node.p,
    operationId: staticOperationId(node),
    instanceId: currentEntityId(ctx),
    originalId,
    start: node.p,
    end: node.end,
    label: `${node.name}()`,
  })
}

export function trackedSolid(geometry: CadKernelHandle, color: RGBA, node: CallNode, ctx: EvalContext): Shape3D {
  // Eager products (hull/boolean) start without a source id. Promote those
  // results so later transforms keep provenance for the generating call.
  const id = ctx.kernel.originalId(geometry)
  const trackedGeometry = id === null || id < 0 ? ctx.kernel.asOriginal(geometry) : geometry
  trackSource(trackedGeometry, node, ctx)
  return { dimension: 3, geometry: trackedGeometry, color, entityId: currentEntityId(ctx) }
}

export function warn(ctx: EvalContext, message: string) {
  if (!ctx.warnings.includes(message)) ctx.warnings.push(message)
}

export function kernelCall<T>(ctx: EvalContext, p: number, run: () => T): T {
  try {
    return run()
  } catch (error) {
    if (error instanceof OpenSCADParseError || error instanceof AbortedError) throw error
    throw positionKernelError(ctx.source, p, error)
  }
}

export function registerStringValue(value: string, ctx: EvalContext, p: number, limitLabel = 'Evaluation'): string {
  ctx.valueBudget.used += Math.max(1, value.length)
  if (ctx.valueBudget.used > MAX_EVALUATED_VALUE_UNITS) {
    evaluationError(ctx, p, `${limitLabel} exceeds the ${MAX_EVALUATED_VALUE_UNITS.toLocaleString()} value-allocation budget`)
  }
  return value
}

export function resolveStableVariable(name: string, ctx: EvalContext): { found: boolean; value: Value } {
  if (name.startsWith('$') && ctx.env.has(name)) {
    return { found: true, value: ctx.env.get(name) }
  }
  let scope = ctx.stableScope
  let visibleBefore = ctx.scopeVisibleBefore ?? Number.POSITIVE_INFINITY
  while (scope !== undefined && scope !== null) {
    const local = scope.resolveLocalVariable(
      name,
      visibleBefore,
      (expression, site) => evalExpression(expression, {
        ...ctx,
        env: site.env,
        stableScope: site.scope,
        scopeVisibleBefore: site.visibleBefore,
      }),
      message => warn(ctx, message),
    )
    if (local.found) return local
    if (scope === ctx.stableScope && ctx.env.has(name)) {
      return { found: true, value: ctx.env.get(name) }
    }
    scope = scope.parent ?? undefined
    visibleBefore = Number.POSITIVE_INFINITY
  }
  if (ctx.env.has(name)) return { found: true, value: ctx.env.get(name) }
  return { found: false, value: undefined }
}

export const {
  stableValueContext,
  enterStableStatementScope,
  evaluateSequentialBindings,
  stableIterable,
  evalListComprehension,
  resolveStableExpressionArguments,
  evalAssertExpression,
  evalEchoExpression,
  evalBinaryExpression,
  evalFunctionCall,
  invokeUserFunction,
} = createStableExpressionEvaluators<EvalContext>({
  evalExpression: (expr, ctx, depth) => evalExpression(expr, ctx, depth),
  evalBuiltin: (expr, ctx, depth) => evalBuiltin(expr, ctx, depth),
  resolveStableVariable: (name, ctx) => resolveStableVariable(name, ctx),
  warn: (ctx, _position, message) => warn(ctx, message),
  scopeWarn: (ctx, _position, message) => warn(ctx, message),
  echoWarn: (ctx, _position, message) => { ctx.warnings.push(message) },
  finiteNumber: (value, ctx, position, label) => finiteNumber(value, ctx, position, label),
})

/**
 * Bind a stable built-in module without re-evaluating argument expressions.
 * Positional slots and named-only special variables are deliberately separate:
 * `$fn` is valid by name but is never a seventh linear_extrude positional slot.
 */
export function evaluateStableBuiltinModuleArguments(
  node: CallNode,
  positionalNames: readonly string[],
  namedOnlyNames: readonly string[],
  ctx: EvalContext,
): Map<string, Value> {
  const args = callExpressionArguments(node)
  const allowed = new Set([...positionalNames, ...namedOnlyNames])
  const resolved = new Map<string, ExpressionArgument>()
  for (const argument of args) {
    const name = argument.name ?? positionalNames.find(parameter => !resolved.has(parameter))
    if (name === undefined) {
      warn(ctx, `Ignoring excess positional argument to ${node.name}()`)
      continue
    }
    if (!allowed.has(name)) {
      warn(ctx, `Ignoring unknown argument ${name} to ${node.name}()`)
      continue
    }
    if (resolved.has(name)) warn(ctx, `Argument ${name} was specified more than once for ${node.name}()`)
    resolved.set(name, argument)
  }

  // OpenSCAD binds the complete call before invoking the module. Evaluate
  // every supplied expression exactly once, including ignored arguments whose
  // echo/assert effects remain observable.
  const evaluated = new Map<ExpressionArgument, Value>()
  for (const argument of args) evaluated.set(argument, evalExpression(argument.value, ctx))
  return new Map([...resolved].map(([name, argument]) => [name, evaluated.get(argument)]))
}

export interface EvaluatedStableCompatibilityArguments {
  readonly authored: readonly ExpressionArgument[]
  readonly values: ReadonlyMap<string, Value>
  readonly evaluated: ReadonlyMap<ExpressionArgument, Value>
}

export function evaluateStableCompatibilityArguments(
  node: CallNode,
  positionalNames: readonly string[],
  namedOnlyNames: readonly string[],
  ctx: EvalContext,
): EvaluatedStableCompatibilityArguments {
  const authored = callExpressionArguments(node)
  const allowed = new Set([...positionalNames, ...namedOnlyNames])
  const resolved = new Map<string, ExpressionArgument>()
  for (const argument of authored) {
    const name = argument.name ?? positionalNames.find(parameter => !resolved.has(parameter))
    if (name === undefined) {
      warn(ctx, `Ignoring excess positional argument to ${node.name}()`)
      continue
    }
    if (!allowed.has(name)) {
      warn(ctx, `Ignoring unknown argument ${name} to ${node.name}()`)
      continue
    }
    if (resolved.has(name)) warn(ctx, `Argument ${name} was specified more than once for ${node.name}()`)
    resolved.set(name, argument)
  }

  const evaluated = new Map<ExpressionArgument, Value>()
  for (const argument of authored) evaluated.set(argument, evalExpression(argument.value, ctx))
  return {
    authored,
    evaluated,
    values: new Map([...resolved].map(([name, argument]) => [name, evaluated.get(argument)])),
  }
}

export function evalExpression(expr: Expr, ctx: EvalContext, depth = 0): Value {
  if (++ctx.budget.ops > MAX_EVAL_OPS) {
    evaluationError(ctx, expr.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
  }
  if (depth >= MAX_EXPRESSION_DEPTH) {
    evaluationError(ctx, expr.p, `Expression exceeds ${MAX_EXPRESSION_DEPTH} evaluated levels`)
  }
  const evaluate = (child: Expr) => evalExpression(child, ctx, depth + 1)
  switch (expr.kind) {
    case 'literal': return expr.value
    case 'identifier': {
      if (expr.name === 'PI') return Math.PI
      const resolved = isStableProfile(ctx)
        ? resolveStableVariable(expr.name, ctx)
        : { found: ctx.env.has(expr.name), value: ctx.env.get(expr.name) }
      if (!resolved.found) {
        if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, `Unknown variable ${expr.name}`)
        warn(ctx, `Ignoring unknown variable '${expr.name}'`)
        return undefined
      }
      return resolved.value
    }
    case 'vector': {
      if (!isStableProfile(ctx)) return registerArrayValue(expr.items.map(evaluate), ctx, expr.p)
      const values: Value[] = []
      for (const item of expr.items) {
        const value = evaluate(item)
        if (isListComprehensionExpression(item) && Array.isArray(value)) {
          appendComprehensionValues(values, value, item, ctx)
        } else values.push(value)
      }
      return registerArrayValue(values, ctx, expr.p)
    }
    case 'range': {
      if (isStableProfile(ctx)) {
        const start = evaluate(expr.start)
        const end = evaluate(expr.end)
        const step = expr.step === undefined ? 1 : evaluate(expr.step)
        if (typeof start !== 'number' || typeof step !== 'number' || typeof end !== 'number'
          || ![start, step, end].every(Number.isFinite)) {
          warn(ctx, 'Invalid range bounds produce undef')
          return undefined
        }
        if (step > 0 && start > end) {
          warn(ctx, 'begin is greater than the end, but step is positive')
        } else if (step < 0 && start < end) {
          warn(ctx, 'begin is smaller than the end, but step is negative')
        }
        return { kind: 'range-value', start, step, end }
      }
      const start = finiteNumber(evaluate(expr.start), ctx, expr.p, 'range start')
      const end = finiteNumber(evaluate(expr.end), ctx, expr.p, 'range end')
      const step = expr.step ? finiteNumber(evaluate(expr.step), ctx, expr.p, 'range step') : 1
      if (step === 0) evaluationError(ctx, expr.p, 'Range step cannot be zero')
      const values: number[] = []
      const forward = step > 0
      for (let value = start; forward ? value <= end + 1e-10 : value >= end - 1e-10; value += step) {
        values.push(value)
        if (values.length > MAX_RANGE_ITEMS) evaluationError(ctx, expr.p, `Range exceeds ${MAX_RANGE_ITEMS.toLocaleString()} items`)
      }
      return registerArrayValue(values, ctx, expr.p)
    }
    case 'unary': {
      const value = evaluate(expr.value)
      if (isStableProfile(ctx)) return openScadUnary(expr.op, value, stableValueContext(ctx, expr.p))
      if (expr.op === TT.Not) return !truthy(value)
      const number = finiteNumber(value, ctx, expr.p, 'unary operand')
      return expr.op === TT.Minus ? -number : number
    }
    case 'binary': return evalBinaryExpression(expr, ctx, depth)
    case 'ternary': return (isStableProfile(ctx) ? openScadTruthy(evaluate(expr.test)) : truthy(evaluate(expr.test)))
      ? evaluate(expr.yes)
      : evaluate(expr.no)
    case 'index': {
      const value = evaluate(expr.value)
      if (isStableProfile(ctx)) {
        return openScadIndex(value, evaluate(expr.index), stableValueContext(ctx, expr.p))
      }
      const index = Math.trunc(finiteNumber(evaluate(expr.index), ctx, expr.p, 'index'))
      if (Array.isArray(value) || typeof value === 'string') return value[index] as Value
      evaluationError(ctx, expr.p, 'Only vectors and strings can be indexed')
    }
    case 'member': {
      const value = evaluate(expr.value)
      if (isStableProfile(ctx)) return openScadMember(value, expr.name, stableValueContext(ctx, expr.p))
      if (Array.isArray(value)) {
        const index = ({ x: 0, y: 1, z: 2 } as const)[expr.name as 'x' | 'y' | 'z']
        if (index !== undefined) return value[index]
      }
      evaluationError(ctx, expr.p, `Value has no member ${expr.name}`)
    }
    case 'function': {
      const value: FunctionValue = {
        kind: 'function-value',
        name: null,
        params: expr.params,
        body: expr.body,
        closure: new Map(ctx.env),
      }
      if (isStableProfile(ctx) && ctx.stableScope !== undefined) {
        return { ...value, lexicalScope: ctx.stableScope } as StableFunctionValue
      }
      return value
    }
    case 'call': return evalFunctionCall(expr, ctx, depth)
    case 'let': {
      if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, 'let expression is not supported')
      const env = evaluateSequentialBindings(expr.args, ctx, depth + 1)
      return evalExpression(expr.body, stableOverlayContext(ctx, env), depth + 1)
    }
    case 'assert':
      if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, 'assert expression is not supported')
      return evalAssertExpression(expr, ctx, depth)
    case 'echo':
      if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, 'echo expression is not supported')
      return evalEchoExpression(expr, ctx, depth)
    case 'lc-for':
    case 'lc-for-c':
    case 'lc-if':
    case 'lc-let':
    case 'lc-each':
      if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, 'list comprehension is not supported')
      return registerArrayValue(evalListComprehension(expr, ctx, depth), ctx, expr.p, 'list comprehension')
  }
}

export function compatibilityString(value: Value): string {
  if (value === undefined) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return formatOpenScadValue(value)
}

export function evalDxfQueryBuiltin(
  expr: Extract<Expr, { kind: 'call' }>,
  ctx: EvalContext,
  depth: number,
): Value {
  const name = expr.name as 'dxf_dim' | 'dxf_cross'
  const supported = new Set(name === 'dxf_dim'
    ? ['file', 'layer', 'origin', 'scale', 'name']
    : ['file', 'layer', 'origin', 'scale'])
  const values = new Map<string, Value>()
  for (const argument of expr.args) {
    const value = evalExpression(argument.value, ctx, depth + 1)
    const argumentName = argument.name ?? ''
    if (!supported.has(argumentName)) {
      warn(ctx, `${name}(..., ${argumentName}=...) is not supported`)
      continue
    }
    values.set(argumentName, value)
  }

  const specifier = compatibilityString(values.get('file'))
  if (ctx.project === undefined || ctx.importAssets === undefined) {
    warn(ctx, `Can't open DXF file '${specifier}'!`)
    return undefined
  }
  const sourcePath = ctx.project.entrypoint
  const rawOrigin = values.get('origin')
  const rawScale = values.get('scale')
  const common = {
    layer: compatibilityString(values.get('layer')),
    ...(values.has('origin')
      ? { origin: rawOrigin as unknown as readonly [number, number] }
      : {}),
    ...(typeof rawScale === 'number' ? { scale: rawScale } : {}),
  }
  const result = name === 'dxf_dim'
    ? queryPreparedOpenScadDxfDimension(
        ctx.project,
        ctx.importAssets,
        sourcePath,
        specifier,
        { ...common, name: compatibilityString(values.get('name')) },
      )
    : queryPreparedOpenScadDxfCross(
        ctx.project,
        ctx.importAssets,
        sourcePath,
        specifier,
        common,
      )
  for (const diagnostic of result.diagnostics) warn(ctx, diagnostic.message)
  if (result.value === undefined || typeof result.value === 'number') return result.value
  return [result.value[0], result.value[1]]
}

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

export function arg(node: CallNode, name: string, position: number, fallback: Value, ctx: EvalContext): Value {
  const expression = node.args[name] ?? node.args[`_${position}`]
  return expression ? evalExpression(expression, ctx) : fallback
}
