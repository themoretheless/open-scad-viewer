import { TT, hasOpenScadViewportModifier } from '../openscadCompiler'
import type { CallNode, Expr, ExpressionArgument, FunctionNode, FunctionValue, ModuleNode, Statement, Value } from '../openscadCompiler'
import { StableBuiltinValueError, enterStableStatementScope, evalAssertExpression, evalBinaryExpression, evalEchoExpression, evalFunctionCall, evalListComprehension, evaluateSequentialBindings, stableValueContext } from '../openscadSemanticLowerer'
import type { EvalContext, EvaluatedStableCompatibilityArguments, PassedCallChildren, SemanticShape } from '../openscadSemanticLowerer'
import { MAX_EVAL_OPS, MAX_EXPRESSION_DEPTH, MAX_RANGE_ITEMS, MAX_VALUE_ELEMENTS, appendComprehensionValues, compactDiagnosticText, evaluationError, isFunctionValue, isListComprehensionExpression, isStableProfile, registerArrayValue, stableOverlayContext } from '../openScadStableExpressionEval'
import type { StableFunctionValue } from '../openScadStableExpressionEval'
import { OpenSCADParseError } from '../openscadErrors'
import { createDeferredOpenScadBuiltinArguments, evaluateOpenScadBuiltinFunction } from '../openScadBuiltinFunctions'
import type { OpenScadBuiltinValue } from '../openScadBuiltinFunctions'
import type { SemanticDynamicSlot } from '../../core/semanticProgram'
import { canonicalNumber, finiteNumber, registerStringValue, resolveStableVariable, stableWarning, truthy } from './semanticValueType'
import { evalPreparedNodes, semanticSlot } from './evalPreparedNodes'
import { openScadIndex, openScadMember, openScadTruthy, openScadUnary } from '../openScadValueSemantics'

export function evalExpression(expr: Expr, ctx: EvalContext, depth = 0): Value {
  if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, expr.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
  if (depth >= MAX_EXPRESSION_DEPTH) evaluationError(ctx, expr.p, `Expression exceeds ${MAX_EXPRESSION_DEPTH} evaluated levels`)
  const evaluate = (child: Expr) => evalExpression(child, ctx, depth + 1)
  switch (expr.kind) {
    case 'literal': return expr.value
    case 'identifier': {
      if (expr.name === 'PI') return Math.PI
      const resolved = isStableProfile(ctx)
        ? resolveStableVariable(expr.name, ctx, expr.p)
        : { found: ctx.env.has(expr.name), value: ctx.env.get(expr.name) }
      if (!resolved.found) {
        if (!isStableProfile(ctx)) evaluationError(ctx, expr.p, `Unknown variable ${expr.name}`)
        stableWarning(ctx, expr.p, `Ignoring unknown variable '${expr.name}'`)
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
          stableWarning(ctx, expr.p, 'Invalid range bounds produce undef')
          return undefined
        }
        if (step > 0 && start > end) {
          stableWarning(ctx, expr.p, 'begin is greater than the end, but step is positive')
        } else if (step < 0 && start < end) {
          stableWarning(ctx, expr.p, 'begin is smaller than the end, but step is negative')
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
        values.push(canonicalNumber(value))
        if (values.length > MAX_RANGE_ITEMS) evaluationError(ctx, expr.p, `Range exceeds ${MAX_RANGE_ITEMS.toLocaleString()} items`)
      }
      return registerArrayValue(values, ctx, expr.p)
    }
    case 'unary': {
      const value = evaluate(expr.value)
      if (isStableProfile(ctx)) return openScadUnary(expr.op, value, stableValueContext(ctx, expr.p))
      if (expr.op === TT.Not) return !truthy(value)
      const number = finiteNumber(value, ctx, expr.p, 'unary operand')
      return canonicalNumber(expr.op === TT.Minus ? -number : number)
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

export function evalBuiltin(expr: Extract<Expr, { kind: 'call' }>, ctx: EvalContext, depth: number): Value {
  const name = expr.name
  if (name === null) evaluationError(ctx, expr.p, 'Expression is not callable')
  if (name === 'assert') evaluationError(ctx, expr.p, 'Expression-form assert() is not supported; use statement assert()')
  if (isStableProfile(ctx) && (name === 'dxf_dim' || name === 'dxf_cross')) {
    semanticProjectAssetRequired(ctx, expr.p, undefined, `${name}()`)
  }
  if (!isStableProfile(ctx) && expr.args.some(argument => argument.name !== undefined)) {
    evaluationError(ctx, expr.p, `${name}() does not accept named arguments in this engine revision`)
  }
  const evaluateArgument = (argument: ExpressionArgument): Value => {
    // OpenSCAD deliberately permits probing an undeclared bare name with
    // is_undef() without emitting the ordinary unknown-variable warning.
    if (isStableProfile(ctx) && name === 'is_undef' && expr.args.length === 1
      && argument.value.kind === 'identifier') {
      if (argument.value.name === 'PI') return Math.PI
      const resolved = resolveStableVariable(argument.value.name, ctx, argument.value.p)
      return resolved.found ? resolved.value : undefined
    }
    return evalExpression(argument.value, ctx, depth + 1)
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
        warning: message => stableWarning(ctx, expr.p, message, 'W_OPENSCAD_BUILTIN'),
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
    stableWarning(ctx, expr.p, error.message, 'W_OPENSCAD_BUILTIN')
    return undefined
  }
  if (!result.recognized) evaluationError(ctx, expr.p, `Unsupported function ${name}()`)
  return result.value as Value
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

/** Bind legacy built-in modules from the complete authored argument stream. */
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
      stableWarning(ctx, argument.p, `Ignoring excess positional argument to ${node.name}()`)
      continue
    }
    if (!allowed.has(name)) {
      stableWarning(ctx, argument.p, `Ignoring unknown argument ${name} to ${node.name}()`)
      continue
    }
    if (resolved.has(name)) {
      stableWarning(ctx, argument.p, `Argument ${name} was specified more than once for ${node.name}()`)
    }
    resolved.set(name, argument)
  }

  // Stable module calls evaluate every authored argument exactly once. The
  // preserved stream matters for duplicate names and ignored expressions.
  const evaluated = new Map<ExpressionArgument, Value>()
  for (const argument of authored) evaluated.set(argument, evalExpression(argument.value, ctx))
  return Object.freeze({
    authored: Object.freeze(authored),
    evaluated,
    values: new Map([...resolved].map(([name, argument]) => [name, evaluated.get(argument)])),
  })
}

export function semanticProjectAssetRequired(
  ctx: EvalContext,
  position: number,
  end: number | undefined,
  callable: string,
): never {
  throw new OpenSCADParseError(
    ctx.source,
    position,
    `${callable} requires an OpenSCAD project asset context; single-source semantic lowering cannot resolve project files.`,
    'E_IMPORT_PROJECT_REQUIRED',
    end,
  )
}

export function stableCompatibilityDeprecation(
  node: CallNode,
  ctx: EvalContext,
  replacement: string,
): void {
  const message = node.name === 'child'
    ? 'child() will be removed in future releases. Use children() instead.'
    : `The ${node.name}() module will be removed in future releases. Use ${replacement} instead.`
  stableWarning(ctx, node.p, message, 'W_OPENSCAD_DEPRECATED')
}

export function bindAssertArguments(node: CallNode, ctx: EvalContext): { condition: Expr; conditionText: string; message?: Expr } {
  const keys = Object.keys(node.args)
  const unknown = keys.find(key => node.argKinds[key] === 'named'
    ? key !== 'condition' && key !== 'message'
    : key !== '_0' && key !== '_1')
  if (unknown) evaluationError(ctx, node.p, `assert() does not accept argument ${unknown}`)
  const has = (key: string) => Object.hasOwn(node.args, key)
  if (has('_0') && has('condition')) evaluationError(ctx, node.p, 'assert() condition was provided more than once')
  if (has('_1') && has('message')) evaluationError(ctx, node.p, 'assert() message was provided more than once')
  const conditionKey = has('condition') ? 'condition' : has('_0') ? '_0' : null
  if (!conditionKey) evaluationError(ctx, node.p, 'assert() requires a condition')
  const messageKey = has('message') ? 'message' : has('_1') ? '_1' : null
  const span = node.argSpans[conditionKey]
  return {
    condition: node.args[conditionKey],
    conditionText: compactDiagnosticText(span ? ctx.source.slice(span.start, span.end) : 'condition'),
    message: messageKey ? node.args[messageKey] : undefined,
  }
}

export function collectModules(nodes: readonly Statement[], modules: Map<string, ModuleNode>): void {
  for (const node of nodes) {
    if (node.type === 'module') {
      modules.set(node.name, node)
      collectModules(node.children, modules)
    } else if (node.type === 'call') {
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

export function detachedChildrenContinuationMap(
  root: CallNode,
  passed: Readonly<PassedCallChildren> | undefined,
): ReadonlyMap<CallNode, Readonly<PassedCallChildren>> | undefined {
  if ((root.name !== 'children' && root.name !== 'child') || passed === undefined) return undefined
  const result = new Map<CallNode, Readonly<PassedCallChildren>>([[root, passed]])
  const visited = new Set<Readonly<PassedCallChildren>>()
  let current: Readonly<PassedCallChildren> | undefined = passed
  while (current !== undefined && !visited.has(current)) {
    visited.add(current)
    const continuation: Readonly<PassedCallChildren> | undefined = current.continuation
    if (continuation === undefined) break
    const collectConsumers = (statements: readonly Statement[]): void => {
      for (const statement of statements) {
        if (statement.type !== 'call') continue
        if (statement.name === 'children' || statement.name === 'child') result.set(statement, continuation)
        collectConsumers(statement.children)
        collectConsumers(statement.alternative)
      }
    }
    collectConsumers(current.statements)
    current = continuation
  }
  return result
}

export function evalNodes(nodes: readonly Statement[], parent: EvalContext, scoped = true): SemanticShape[] {
  const ctx: EvalContext = isStableProfile(parent)
    ? enterStableStatementScope(nodes, parent)
    : { ...parent, env: scoped ? new Map(parent.env) : parent.env }
  return evalPreparedNodes(nodes, ctx)
}

export function viewportSemanticSlots(node: CallNode, ctx: EvalContext): SemanticDynamicSlot[] {
  if (!isStableProfile(ctx) || ctx.viewportRootOwner === node) return []
  const slots: SemanticDynamicSlot[] = []
  if (hasOpenScadViewportModifier(node, 'highlight')) {
    slots.push(semanticSlot('$viewport-highlight', true))
  }
  if (hasOpenScadViewportModifier(node, 'background')) {
    slots.push(semanticSlot('$viewport-background', true))
  }
  return slots
}

export function viewportRootActivates(node: CallNode, shapes: readonly SemanticShape[]): boolean {
  if (shapes.length > 0) return true
  // These effect/branch calls do not contribute an empty CSG container in
  // OpenSCAD 2021.01. Other evaluated module instantiations do, so a `!` on
  // union(), transform{}, group{}, for(empty), or an empty user module still
  // wins and deliberately yields an empty top-level result.
  return !['if', 'assert', 'echo', 'children', 'child'].includes(node.name)
}
