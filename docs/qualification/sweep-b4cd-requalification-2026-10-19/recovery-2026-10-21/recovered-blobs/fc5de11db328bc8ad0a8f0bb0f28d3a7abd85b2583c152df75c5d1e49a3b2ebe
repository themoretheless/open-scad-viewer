/**
 * Shared implementation of the OpenSCAD "stable" expression evaluator used by
 * both the direct evaluator (openscadParser) and the semantic lowerer
 * (openscadSemanticLowerer). The two hosts historically carried byte-identical
 * copies of these functions; the only intentional differences are the warning
 * sinks (plain string list vs. structured semantic diagnostics) and a few
 * context details, all of which are parameterized through
 * {@link StableExpressionEvalHost}.
 */
import {
  TT,
  type Expr,
  type ExpressionArgument,
  type FunctionNode,
  type FunctionValue,
  type ListComprehensionExpression,
  type OpenScadLanguageProfile,
  type Statement,
  type Value,
} from './openscadCompiler'
import { OpenSCADParseError } from './openscadErrors'
import { OpenScadStableScope } from './openScadStableScope'
import {
  formatOpenScadValue,
  isOpenScadRange,
  materializeOpenScadRange,
  openScadBinary,
  openScadTruthy,
  type OpenScadValueSemanticsContext,
} from './openScadValueSemantics'

export const MAX_EXPRESSION_DEPTH = 256
export const MAX_EVAL_DEPTH = 128
export const MAX_EVALUATED_VALUE_UNITS = 500_000
export const MAX_RANGE_ITEMS = 10_000
export const MAX_EVAL_OPS = 1_000_000
export const MAX_VALUE_ELEMENTS = 1_000_000

/** Structural subset of an evaluation context the shared evaluator relies on. */
export interface StableExpressionEvalContext {
  readonly source: string
  readonly languageProfile: OpenScadLanguageProfile
  readonly env: Map<string, Value>
  readonly functions: Map<string, FunctionNode>
  readonly budget: { ops: number }
  readonly functionStack: readonly string[]
  readonly valueBudget: { used: number }
  readonly valueWeights: WeakMap<Value[], number>
  readonly valueDepths: WeakMap<Value[], number>
  readonly stableScope?: OpenScadStableScope
  readonly scopeVisibleBefore?: number
}

export interface StableFunctionValue extends FunctionValue {
  readonly lexicalScope: OpenScadStableScope
}

/**
 * Host-provided hooks. Every callback must reproduce the calling module's
 * historical behavior exactly; the shared code only owns control flow.
 */
export interface StableExpressionEvalHost<C extends StableExpressionEvalContext> {
  /** The host's own expression evaluator (shared functions recurse through it). */
  readonly evalExpression: (expr: Expr, ctx: C, depth: number) => Value
  /** The host's built-in function fallback used by evalFunctionCall. */
  readonly evalBuiltin: (expr: Extract<Expr, { kind: 'call' }>, ctx: C, depth: number) => Value
  /** The host's stable variable resolver (position semantics differ per host). */
  readonly resolveStableVariable: (name: string, ctx: C, position: number) => { found: boolean; value: Value }
  /** Ordinary warning sink (parser: deduped string list; lowerer: positioned diagnostic). */
  readonly warn: (ctx: C, position: number, message: string) => void
  /** Lexical-scope warning sink (lowerer uses code W_OPENSCAD_SCOPE). */
  readonly scopeWarn: (ctx: C, position: number, message: string) => void
  /** ECHO sink — never deduplicated in either host. */
  readonly echoWarn: (ctx: C, position: number, message: string) => void
  /** The host's finite-number coercion (the lowerer canonicalizes -0). */
  readonly finiteNumber: (value: Value, ctx: C, position: number, label: string) => number
  /**
   * Optional canonicalization applied to the non-stable arithmetic result.
   * The semantic lowerer passes canonicalNumber (maps -0 to 0); the direct
   * evaluator omits it and returns the raw IEEE result.
   */
  readonly canonicalNumber?: (value: number) => number
}

export function isStableProfile(ctx: StableExpressionEvalContext): boolean {
  return ctx.languageProfile === 'openscad/stable-2021.01'
}

export function evaluationError(ctx: StableExpressionEvalContext, p: number, message: string): never {
  throw new OpenSCADParseError(ctx.source, p, message)
}

function valueWeight(value: Value, ctx: StableExpressionEvalContext): number {
  if (Array.isArray(value)) return ctx.valueWeights.get(value) ?? value.length + 1
  return typeof value === 'string' ? Math.max(1, value.length) : 1
}

function valueDepth(value: Value, ctx: StableExpressionEvalContext): number {
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

function truthy(value: Value): boolean {
  return value !== false && value !== undefined && value !== 0 && value !== ''
    && (!Array.isArray(value) || value.length > 0)
}

function deepEqual(left: Value, right: Value): boolean {
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => deepEqual(value, right[index]))
  }
  return left === right
}

export interface StableExpressionEvaluators<C extends StableExpressionEvalContext> {
  readonly stableValueContext: (ctx: C, position: number) => OpenScadValueSemanticsContext
  readonly enterStableStatementScope: (statements: readonly Statement[], parent: C) => C
  readonly evaluateSequentialBindings: (args: readonly ExpressionArgument[], ctx: C, depth: number) => Map<string, Value>
  readonly stableIterable: (value: Value, ctx: C, position: number) => Value[]
  readonly evalListComprehension: (expr: ListComprehensionExpression, ctx: C, depth: number) => Value[]
  readonly resolveStableExpressionArguments: (
    args: readonly ExpressionArgument[],
    parameterNames: readonly string[],
    ctx: C,
  ) => Map<string, ExpressionArgument>
  readonly evalAssertExpression: (expr: Extract<Expr, { kind: 'assert' }>, ctx: C, depth: number) => Value
  readonly evalEchoExpression: (expr: Extract<Expr, { kind: 'echo' }>, ctx: C, depth: number) => Value
  readonly evalBinaryExpression: (expr: Extract<Expr, { kind: 'binary' }>, ctx: C, depth: number) => Value
  readonly evalFunctionCall: (expr: Extract<Expr, { kind: 'call' }>, ctx: C, depth: number) => Value
  readonly invokeUserFunction: (fn: FunctionValue, args: readonly ExpressionArgument[], ctx: C, depth: number) => Value
}

export function createStableExpressionEvaluators<C extends StableExpressionEvalContext>(
  host: StableExpressionEvalHost<C>,
): StableExpressionEvaluators<C> {
  const stableValueContext = (ctx: C, position: number): OpenScadValueSemanticsContext => ({
    warn: message => host.warn(ctx, position, message),
    maxRangeItems: MAX_RANGE_ITEMS,
    registerArray: (values, label) => registerArrayValue(values, ctx, position, label),
  })

  const enterStableStatementScope = (statements: readonly Statement[], parent: C): C => {
    const scope = new OpenScadStableScope(statements, parent.stableScope ?? null, parent.env)
    const ctx: C = {
      ...parent,
      env: scope.env,
      stableScope: scope,
      scopeVisibleBefore: Number.POSITIVE_INFINITY,
    }
    for (const name of scope.dynamicVariableNames()) {
      const resolved = scope.resolveLocalVariable(
        name,
        Number.POSITIVE_INFINITY,
        (expression, site) => host.evalExpression(expression, {
          ...ctx,
          env: site.env,
          stableScope: site.scope,
          scopeVisibleBefore: site.visibleBefore,
        }, 0),
        message => host.scopeWarn(ctx, statements[0]?.p ?? 0, message),
      )
      if (resolved.found) scope.env.set(name, resolved.value)
    }
    return ctx
  }

  const evaluateSequentialBindings = (
    args: readonly ExpressionArgument[],
    ctx: C,
    depth: number,
  ): Map<string, Value> => {
    const env = new Map(ctx.env)
    const assigned = new Set<string>()
    for (const argument of args) {
      const value = host.evalExpression(argument.value, stableOverlayContext(ctx, env), depth + 1)
      if (argument.name === undefined) {
        host.warn(ctx, argument.p, `Ignoring assignment without variable name ${formatOpenScadValue(value)}`)
        continue
      }
      if (assigned.has(argument.name)) {
        host.warn(ctx, argument.p, `Ignoring duplicate variable assignment ${argument.name} = ${formatOpenScadValue(value)}`)
        continue
      }
      assigned.add(argument.name)
      env.set(argument.name, value)
    }
    return env
  }

  const stableIterable = (value: Value, ctx: C, position: number): Value[] => {
    if (isOpenScadRange(value)) return materializeOpenScadRange(value, stableValueContext(ctx, position))
    if (Array.isArray(value)) return value
    if (typeof value === 'string') {
      return registerArrayValue(Array.from(value), ctx, position, 'string iteration')
    }
    return value === undefined ? [] : [value]
  }

  const evalComprehensionElement = (expr: Expr, ctx: C, depth: number): Value[] => {
    const value = host.evalExpression(expr, ctx, depth + 1)
    return isListComprehensionExpression(expr) && Array.isArray(value) ? value : [value]
  }

  const evalListComprehension = (
    expr: ListComprehensionExpression,
    ctx: C,
    depth: number,
  ): Value[] => {
    const output: Value[] = []
    switch (expr.kind) {
      case 'lc-each':
        return stableIterable(host.evalExpression(expr.value, ctx, depth + 1), ctx, expr.p)
      case 'lc-if': {
        const selected = openScadTruthy(host.evalExpression(expr.condition, ctx, depth + 1))
          ? expr.yes
          : expr.no
        return selected === undefined ? output : evalComprehensionElement(selected, ctx, depth + 1)
      }
      case 'lc-let': {
        const env = evaluateSequentialBindings(expr.args, ctx, depth + 1)
        return evalListComprehension(expr.body, stableOverlayContext(ctx, env), depth + 1)
      }
      case 'lc-for': {
        const visit = (bindingIndex: number, iterationContext: C): void => {
          if (bindingIndex >= expr.args.length) {
            appendComprehensionValues(
              output,
              evalComprehensionElement(expr.body, iterationContext, depth + 1),
              expr,
              ctx,
            )
            return
          }
          const binding = expr.args[bindingIndex]
          const iterable = stableIterable(
            host.evalExpression(binding.value, iterationContext, depth + 1),
            iterationContext,
            binding.p,
          )
          if (binding.name === undefined) {
            host.warn(ctx, binding.p, 'Ignoring for() iterator without variable name')
            return
          }
          for (const value of iterable) {
            if (++ctx.budget.ops > MAX_EVAL_OPS) {
              evaluationError(ctx, expr.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
            }
            const env = new Map(iterationContext.env)
            env.set(binding.name, value)
            visit(bindingIndex + 1, stableOverlayContext(iterationContext, env))
          }
        }
        visit(0, ctx)
        return output
      }
      case 'lc-for-c': {
        let iterationContext: C = {
          ...ctx,
          env: evaluateSequentialBindings(expr.init, ctx, depth + 1),
        }
        iterationContext = stableOverlayContext(ctx, iterationContext.env)
        while (openScadTruthy(host.evalExpression(expr.condition, iterationContext, depth + 1))) {
          if (++ctx.budget.ops > MAX_EVAL_OPS) {
            evaluationError(ctx, expr.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
          }
          appendComprehensionValues(
            output,
            evalComprehensionElement(expr.body, iterationContext, depth + 1),
            expr,
            ctx,
          )
          iterationContext = stableOverlayContext(
            iterationContext,
            evaluateSequentialBindings(expr.update, iterationContext, depth + 1),
          )
        }
        return output
      }
    }
  }

  const resolveStableExpressionArguments = (
    args: readonly ExpressionArgument[],
    parameterNames: readonly string[],
    ctx: C,
  ): Map<string, ExpressionArgument> => {
    const resolved = new Map<string, ExpressionArgument>()
    const parameters = new Set(parameterNames)
    for (const argument of args) {
      const name = argument.name ?? parameterNames.find(parameter => !resolved.has(parameter))
      if (name === undefined) {
        host.warn(ctx, argument.p, 'Ignoring excess positional argument')
        continue
      }
      if (!parameters.has(name)) {
        host.warn(ctx, argument.p, `Ignoring unknown argument ${name}`)
        continue
      }
      if (argument.name !== undefined && resolved.has(name)) {
        host.warn(ctx, argument.p, `Argument ${name} was specified more than once`)
      }
      resolved.set(name, argument)
    }
    return resolved
  }

  const evalAssertExpression = (
    expr: Extract<Expr, { kind: 'assert' }>,
    ctx: C,
    depth: number,
  ): Value => {
    const resolved = resolveStableExpressionArguments(expr.args, ['condition', 'message'], ctx)
    const conditionArgument = resolved.get('condition')
    const messageArgument = resolved.get('message')
    const condition = conditionArgument
      ? host.evalExpression(conditionArgument.value, ctx, depth + 1)
      : undefined
    const message = messageArgument
      ? host.evalExpression(messageArgument.value, ctx, depth + 1)
      : undefined
    if (!openScadTruthy(condition)) {
      const conditionText = conditionArgument
        ? compactDiagnosticText(ctx.source.slice(conditionArgument.p, conditionArgument.end))
        : 'undef'
      const detail = messageArgument ? `: ${compactDiagnosticText(formatOpenScadValue(message))}` : ''
      evaluationError(ctx, expr.p, `Assertion '${conditionText}' failed${detail}`)
    }
    return expr.body === undefined ? undefined : host.evalExpression(expr.body, ctx, depth + 1)
  }

  const evalEchoExpression = (
    expr: Extract<Expr, { kind: 'echo' }>,
    ctx: C,
    depth: number,
  ): Value => {
    const values = expr.args.map(argument => {
      const value = formatOpenScadValue(host.evalExpression(argument.value, ctx, depth + 1))
      return argument.name === undefined ? value : `${argument.name} = ${value}`
    })
    host.echoWarn(ctx, expr.p, `ECHO:${values.length ? ` ${values.join(', ')}` : ''}`)
    return expr.body === undefined ? undefined : host.evalExpression(expr.body, ctx, depth + 1)
  }

  const evalBinaryExpression = (
    expr: Extract<Expr, { kind: 'binary' }>,
    ctx: C,
    depth: number,
  ): Value => {
    const evaluate = (child: Expr) => host.evalExpression(child, ctx, depth + 1)
    if (isStableProfile(ctx)) {
      const left = evaluate(expr.left)
      if (expr.op === TT.And && !openScadTruthy(left)) return false
      if (expr.op === TT.Or && openScadTruthy(left)) return true
      return openScadBinary(expr.op, left, evaluate(expr.right), stableValueContext(ctx, expr.p))
    }
    if (expr.op === TT.And) return truthy(evaluate(expr.left)) && truthy(evaluate(expr.right))
    if (expr.op === TT.Or) return truthy(evaluate(expr.left)) || truthy(evaluate(expr.right))
    const left = evaluate(expr.left)
    const right = evaluate(expr.right)
    if (expr.op === TT.EqEq) return deepEqual(left, right)
    if (expr.op === TT.NotEq) return !deepEqual(left, right)
    if ([TT.Lt, TT.Gt, TT.LtEq, TT.GtEq].includes(expr.op)) {
      const a = host.finiteNumber(left, ctx, expr.p, 'comparison operand')
      const b = host.finiteNumber(right, ctx, expr.p, 'comparison operand')
      if (expr.op === TT.Lt) return a < b
      if (expr.op === TT.Gt) return a > b
      if (expr.op === TT.LtEq) return a <= b
      return a >= b
    }
    const a = host.finiteNumber(left, ctx, expr.p, 'arithmetic operand')
    const b = host.finiteNumber(right, ctx, expr.p, 'arithmetic operand')
    let result: number
    if (expr.op === TT.Plus) result = a + b
    else if (expr.op === TT.Minus) result = a - b
    else if (expr.op === TT.Star) result = a * b
    else if (expr.op === TT.Slash) result = a / b
    else if (expr.op === TT.Percent) result = a % b
    else result = a ** b
    if (!Number.isFinite(result)) evaluationError(ctx, expr.p, 'Expression produced a non-finite number')
    return host.canonicalNumber === undefined ? result : host.canonicalNumber(result)
  }

  const evalFunctionCall = (expr: Extract<Expr, { kind: 'call' }>, ctx: C, depth: number): Value => {
    if (expr.name !== null) {
      const declaration = isStableProfile(ctx)
        ? ctx.stableScope?.functionDeclaration(expr.name)
        : undefined
      const definition = declaration?.node ?? ctx.functions.get(expr.name)
      if (definition && (!isStableProfile(ctx) || declaration !== undefined)) {
        const value: FunctionValue = {
          kind: 'function-value',
          name: definition.name,
          params: definition.params,
          body: definition.body,
          closure: new Map(declaration?.scope.env ?? ctx.env),
        }
        return invokeUserFunction(
          declaration === undefined
            ? value
            : { ...value, lexicalScope: declaration.scope } as StableFunctionValue,
          expr.args,
          ctx,
          depth,
        )
      }
      const resolved = isStableProfile(ctx)
        ? host.resolveStableVariable(expr.name, ctx, expr.p)
        : { found: ctx.env.has(expr.name), value: ctx.env.get(expr.name) }
      if (resolved.found && isFunctionValue(resolved.value)) {
        return invokeUserFunction(resolved.value, expr.args, ctx, depth)
      }
      return host.evalBuiltin(expr, ctx, depth)
    }
    const callee = host.evalExpression(expr.callee, ctx, depth + 1)
    if (!isFunctionValue(callee)) evaluationError(ctx, expr.p, 'Expression is not callable')
    return invokeUserFunction(callee, expr.args, ctx, depth)
  }

  const invokeUserFunction = (
    fn: FunctionValue,
    args: readonly ExpressionArgument[],
    ctx: C,
    depth: number,
  ): Value => {
    if (ctx.functionStack.length >= MAX_EVAL_DEPTH) {
      evaluationError(ctx, args[0]?.p ?? fn.body.p, `Evaluation exceeds ${MAX_EVAL_DEPTH} nested function calls`)
    }
    if (isStableProfile(ctx)) {
      const lexicalScope = (fn as Partial<StableFunctionValue>).lexicalScope ?? ctx.stableScope
      if (lexicalScope === undefined) {
        evaluationError(ctx, args[0]?.p ?? fn.body.p, 'Stable function is missing its lexical scope')
      }
      const resolved = resolveStableExpressionArguments(
        args,
        fn.params.map(parameter => parameter.name),
        ctx,
      )
      const callerValues = new Map<ExpressionArgument, Value>()
      for (const argument of args) {
        callerValues.set(argument, host.evalExpression(argument.value, ctx, depth + 1))
      }
      const definitionEnv = new Map(fn.closure)
      overlayDynamicVariables(definitionEnv, ctx.env)
      const definitionContext: C = {
        ...ctx,
        env: definitionEnv,
        stableScope: lexicalScope,
        scopeVisibleBefore: Number.POSITIVE_INFINITY,
      }
      const env = new Map(definitionEnv)
      const parameterValues = new Map<string, Value>()
      for (const parameter of fn.params) {
        const supplied = resolved.get(parameter.name)
        if (supplied !== undefined) parameterValues.set(parameter.name, callerValues.get(supplied))
        else if (parameter.defaultValue !== undefined) {
          parameterValues.set(parameter.name, host.evalExpression(
            parameter.defaultValue,
            { ...definitionContext, env: new Map(definitionEnv) },
            depth + 1,
          ))
        } else parameterValues.set(parameter.name, undefined)
      }
      for (const [name, value] of parameterValues) env.set(name, value)
      return host.evalExpression(fn.body, {
        ...stableOverlayContext(definitionContext, env),
        functionStack: [...ctx.functionStack, fn.name ?? '<anonymous>'],
      }, depth + 1)
    }
    const positional = args.filter(argument => argument.name === undefined)
    const named = new Map(args.filter(argument => argument.name !== undefined)
      .map(argument => [argument.name!, argument] as const))
    const parameterNames = new Set(fn.params.map(parameter => parameter.name))
    for (const name of named.keys()) {
      if (!parameterNames.has(name)) evaluationError(ctx, args.find(argument => argument.name === name)?.p ?? fn.body.p, `Unknown argument ${name}`)
    }
    if (positional.length > fn.params.length) evaluationError(ctx, positional[fn.params.length]?.p ?? fn.body.p, 'Too many function arguments')

    const callerValues = new Map<ExpressionArgument, Value>()
    for (const argument of args) callerValues.set(argument, host.evalExpression(argument.value, ctx, depth + 1))
    const env = new Map(fn.closure)
    for (let index = 0; index < fn.params.length; index++) {
      const parameter = fn.params[index]
      const supplied = named.get(parameter.name) ?? positional[index]
      if (supplied) env.set(parameter.name, callerValues.get(supplied))
      else if (parameter.defaultValue) env.set(parameter.name, host.evalExpression(parameter.defaultValue, { ...ctx, env }, depth + 1))
      else env.set(parameter.name, undefined)
    }
    return host.evalExpression(fn.body, {
      ...ctx,
      env,
      functionStack: [...ctx.functionStack, fn.name ?? '<anonymous>'],
    }, depth + 1)
  }

  return {
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
  }
}
