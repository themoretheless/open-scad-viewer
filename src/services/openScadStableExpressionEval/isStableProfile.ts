import type { Expr, FunctionValue, ListComprehensionExpression, Value } from '../openscadCompiler'
import { MAX_EVALUATED_VALUE_UNITS, MAX_EXPRESSION_DEPTH, MAX_VALUE_ELEMENTS } from '../openScadStableExpressionEval'
import type { StableExpressionEvalContext } from '../openScadStableExpressionEval'
import { OpenSCADParseError } from '../openscadErrors'
import { OpenScadStableScope } from '../openScadStableScope'

export function isStableProfile(ctx: StableExpressionEvalContext): boolean {
  return ctx.languageProfile === 'openscad/stable-2021.01'
}

export function evaluationError(ctx: StableExpressionEvalContext, p: number, message: string): never {
  throw new OpenSCADParseError(ctx.source, p, message)
}

export function valueWeight(value: Value, ctx: StableExpressionEvalContext): number {
  if (Array.isArray(value)) return ctx.valueWeights.get(value) ?? value.length + 1
  return typeof value === 'string' ? Math.max(1, value.length) : 1
}

export function valueDepth(value: Value, ctx: StableExpressionEvalContext): number {
  return Array.isArray(value) ? ctx.valueDepths.get(value) ?? 1 : 0
}

export function registerArrayValue<T extends Value[]>(
  value: T,
  ctx: StableExpressionEvalContext,
  p: number,
  limitLabel = 'Evaluated value',
): T {
  let weight = 1
  let depth = 1
  for (const item of value) {
    weight += valueWeight(item, ctx)
    depth = Math.max(depth, valueDepth(item, ctx) + 1)
    if (weight > MAX_EVALUATED_VALUE_UNITS) {
      evaluationError(ctx, p, `${limitLabel} exceeds ${MAX_EVALUATED_VALUE_UNITS.toLocaleString()} units`)
    }
  }
  if (depth > MAX_EXPRESSION_DEPTH) {
    evaluationError(ctx, p, `Evaluated value exceeds ${MAX_EXPRESSION_DEPTH} nested levels`)
  }
  ctx.valueBudget.used += weight
  if (ctx.valueBudget.used > MAX_EVALUATED_VALUE_UNITS) {
    evaluationError(ctx, p, `${limitLabel} exceeds the ${MAX_EVALUATED_VALUE_UNITS.toLocaleString()} value-allocation budget`)
  }
  ctx.valueWeights.set(value, weight)
  ctx.valueDepths.set(value, depth)
  return value
}

export function stableOverlayContext<C extends StableExpressionEvalContext>(
  ctx: C,
  env: ReadonlyMap<string, Value>,
): C {
  if (!isStableProfile(ctx)) return { ...ctx, env: new Map(env) }
  const scope = new OpenScadStableScope([], ctx.stableScope ?? null, env)
  return {
    ...ctx,
    env: scope.env,
    stableScope: scope,
    scopeVisibleBefore: Number.POSITIVE_INFINITY,
  }
}

export function overlayDynamicVariables(target: Map<string, Value>, source: ReadonlyMap<string, Value>): void {
  for (const [name, value] of source) if (name.startsWith('$')) target.set(name, value)
}

export function isListComprehensionExpression(expr: Expr): expr is ListComprehensionExpression {
  return expr.kind === 'lc-for'
    || expr.kind === 'lc-for-c'
    || expr.kind === 'lc-if'
    || expr.kind === 'lc-let'
    || expr.kind === 'lc-each'
}

export function isFunctionValue(value: Value): value is FunctionValue {
  return !Array.isArray(value) && typeof value === 'object' && value !== null
    && value.kind === 'function-value'
}

export function compactDiagnosticText(value: string, limit = 240): string {
  const compact = value.replace(/\s+/g, ' ').trim()
  return compact.length <= limit ? compact : `${compact.slice(0, limit - 1)}…`
}

export function appendComprehensionValues(
  output: Value[],
  values: readonly Value[],
  expr: Expr,
  ctx: StableExpressionEvalContext,
): void {
  if (output.length + values.length > MAX_VALUE_ELEMENTS) {
    evaluationError(ctx, expr.p, `List comprehension exceeds ${MAX_VALUE_ELEMENTS.toLocaleString()} elements`)
  }
  output.push(...values)
}

export function truthy(value: Value): boolean {
  return value !== false && value !== undefined && value !== 0 && value !== ''
    && (!Array.isArray(value) || value.length > 0)
}

export function deepEqual(left: Value, right: Value): boolean {
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => deepEqual(value, right[index]))
  }
  return left === right
}
