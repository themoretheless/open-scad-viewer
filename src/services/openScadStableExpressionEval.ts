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
export { isStableProfile, evaluationError, registerArrayValue, stableOverlayContext, overlayDynamicVariables, isListComprehensionExpression, isFunctionValue, compactDiagnosticText, appendComprehensionValues } from './openScadStableExpressionEval/isStableProfile'
export { createStableExpressionEvaluators } from './openScadStableExpressionEval/createStableExpressionEvaluators'

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
