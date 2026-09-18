import { boundedSceneEntityId } from '../core/boundedSceneEntityId'
import { evaluateModelGraphGeometry, requireModelGraphChecks } from './modelGraphChecks'
/**
 * Strict, intentionally documented OpenSCAD subset backed by the official geometry kernel.
 *
 * Supported language features: variables, arithmetic/boolean expressions,
 * ranges, for/if/let, statement assertions, user modules and children(). Supported geometry:
 * cube, sphere, cylinder, polyhedron, square, circle, polygon, transforms,
 * union/difference/intersection/hull, linear/rotate extrusion, projection and
 * 2D offset. Unsupported syntax fails loudly instead of rendering a wrong model.
 */
import type { CadKernelHandle, CadKernelOps } from './cadKernelOps'
import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import { geometryAssetId } from '../core/scene'
import type {
  MeshData,
  MeshProvenanceRun,
  MeshSourceReference,
  SceneEntityId,
  SourceOperationId,
} from '../core/mesh'
import { identity, type Mat4 } from './math3d'
import { buildMeshBvh } from './meshBvh'
import { extractSemanticEdges } from './meshTopology'
import { AbortedError, OpenSCADParseError, positionKernelError } from './openscadErrors'
import { createBrepRecordingKernelOps } from './solid/brepRecorder'
import { bindOpenScad, prepareOpenScadFrontEnd } from './openscadBinder'
import {
  createDeferredOpenScadBuiltinArguments,
  evaluateOpenScadBuiltinFunction,
  type OpenScadBuiltinValue,
} from './openScadBuiltinFunctions'
import {
  TT,
  compileOpenSCAD,
  hasOpenScadViewportModifier,
  type CallNode,
  type Expr,
  type ExpressionArgument,
  type FunctionNode,
  type FunctionValue,
  type ListComprehensionExpression,
  type ModuleNode,
  type OpenScadLanguageProfile,
  type Statement,
  type Value,
} from './openscadCompiler'
import {
  formatOpenScadValue,
  isOpenScadRange,
  materializeOpenScadRange,
  openScadBinary,
  openScadIndex,
  openScadMember,
  openScadTruthy,
  openScadUnary,
  type OpenScadValueSemanticsContext,
} from './openScadValueSemantics'
import {
  resolveOpenScadColor,
  resolveOpenScadFragments,
  resolveOpenScadOffset,
  type OpenScadFragmentResolutionInput,
  type OpenScadStableModuleSemanticsContext,
} from './openScadStableModuleSemantics'
import {
  resolveOpenScadCircle,
  resolveOpenScadCube,
  resolveOpenScadCylinder,
  resolveOpenScadPolygon,
  resolveOpenScadPolyhedron,
  resolveOpenScadSphere,
  resolveOpenScadSquare,
  type OpenScadStablePrimitiveContext,
} from './openScadStablePrimitiveSemantics'
import {
  resolveOpenScadMirror,
  resolveOpenScadMultmatrix,
  resolveOpenScadRotate,
  resolveOpenScadScale,
  resolveOpenScadTranslate,
  type OpenScadStableTransformContext,
  type OpenScadStableTransformPlan,
} from './openScadStableTransformSemantics'
import {
  openScadMinkowski2dViaProduct,
  resolveOpenScad2dMultmatrix,
  resolveOpenScadChildrenSelection,
  resolveOpenScadLinearExtrude,
  resolveOpenScadResize,
  resolveOpenScadRotateExtrude,
  type OpenScadStableGeometryContext,
} from './openScadStableGeometrySemantics'
import { OpenScadStableScope } from './openScadStableScope'
import {
  OpenScadProjectError,
  resolveOpenScadProjectPath,
  type OpenScadProject,
} from './openScadProject'
import { compileOpenScadProject } from './openScadProjectCompiler'
import {
  loadPreparedOpenScadImport,
  OpenScadImportError,
  OpenScadImportPositionedError,
  prepareOpenScadImportAssets,
  queryPreparedOpenScadDxfCross,
  queryPreparedOpenScadDxfDimension,
  type OpenScad2021LegacyImportFormat,
  type OpenScadImportForcedAsset,
  type OpenScadImportErrorCode,
  type PreparedOpenScadImportAssets,
} from './openScadImport'
import {
  loadOpenScadSurfaceHeightMap,
  OpenScadSurfaceDataError,
  OpenScadSurfaceError,
  prepareOpenScadSurfaceAssets,
  triangulateOpenScadSurface,
  type PreparedOpenScadSurfaceAssets,
} from './openScadSurface'
import {
  OpenScadTextError,
  OpenScadTextPositionedError,
  prepareOpenScadTextAssets,
  renderOpenScadText,
  type OpenScadTextErrorCode,
  type OpenScadTextParameters,
  type PreparedOpenScadTextAssets,
} from './openScadText'
import { defaultGeometryKernel } from './cadGeometryKernel'
import { createOpenScadStableRuntimeVariables } from './openScadStableRuntime'
import { CSS_COLORS, clamp01, nextColor, resetPalette, type RGBA } from './openscadColors'

export { AbortedError, OpenSCADParseError } from './openscadErrors'

export interface ParseOptions {
  /**
   * Record the exact solid graph beside the polygon evaluation, for the Solid
   * workspace. Off by default: the bounded worker protocol accepts an exact set of
   * result keys, so the field must not appear on the ordinary display route.
   */
  recordExactSolids?: boolean
  /** Versioned frontend profile. The legacy public route remains subset@1. */
  languageProfile?: OpenScadLanguageProfile
  quality?: GeometryQuality
  /** Host animation position exposed as OpenSCAD's dynamic `$t` (0..1). */
  animationTime?: number
  /**
   * Cooperative cancellation probe. The evaluator yields to the event loop
   * periodically (top-level statements and long `for` / `intersection_for`
   * iteration) and consults this callback; when it returns true the evaluation
   * rejects with {@link AbortedError}. A single enormous boolean/kernel call
   * still will not yield mid-call — BuildCoordinator's worker-replacement grace
   * remains that hard boundary.
   */
  shouldAbort?: () => boolean
  /**
   * Liveness hook invoked after each successful yield. A hosting worker can
   * forward it as a (throttled) progress heartbeat so a watchdog can tell
   * "alive but heavy" from "wedged" instead of killing legitimate long builds.
   */
  onYield?: () => void
  /** Injectable monotonic clock for deterministic phase-timing tests. */
  now?: () => number
  /** Injectable macrotask yield used by cooperative-cancellation tests. */
  yieldControl?: () => Promise<void>
}

export type ParseOpenScadProjectOptions = Omit<ParseOptions, 'languageProfile'>

/** @deprecated Prefer the kernel-neutral GeometryEvaluationResult contract. */
export type ParseResult = GeometryEvaluationResult

const MAX_SOURCE_LENGTH = 250_000
const MAX_AST_NODES = 25_000
const MAX_EXPRESSION_DEPTH = 256
const MAX_EVAL_DEPTH = 128
const MAX_EVALUATED_VALUE_UNITS = 500_000
const MAX_RANGE_ITEMS = 10_000
const MAX_SHAPES = 1_000
const MAX_TRIANGLES = 750_000
const MAX_FN = 256
/** Total evaluation steps (expressions + statements + loop iterations) before aborting —
 * bounds nested loops whose bodies produce no shapes, which MAX_SHAPES
 * can never catch (`for(i=[0:9999]) for(j=[0:9999]) x = i+j;`). */
const MAX_EVAL_OPS = 1_000_000
/** Cap on evaluated vector/string length — `a = concat(a, a);` repeated
 * ~40 times otherwise materializes 2^40 elements and OOMs the worker. */
const MAX_VALUE_ELEMENTS = 1_000_000
/** Cap on linear_extrude slices — passed straight into the geometry kernel,
 * which allocates per-slice cross-sections before MAX_TRIANGLES can fire. */
const MAX_EXTRUDE_SLICES = 512

type Vec2 = [number, number]
type Vec3 = [number, number, number]

interface EvalContext {
  kernel: CadKernelOps
  source: string
  project?: OpenScadProject
  importAssets?: PreparedOpenScadImportAssets
  surfaceAssets?: PreparedOpenScadSurfaceAssets
  textAssets?: PreparedOpenScadTextAssets
  languageProfile: OpenScadLanguageProfile
  env: Map<string, Value>
  functions: Map<string, FunctionNode>
  modules: Map<string, ModuleNode>
  warnings: string[]
  quality: GeometryQuality
  sourceReferences: Map<number, MeshSourceReference>
  callChildren?: Statement[] | PassedCallChildren
  functionStack: string[]
  moduleStack: string[]
  stableScope?: OpenScadStableScope
  scopeVisibleBefore?: number
  depth: number
  /** Shared mutable evaluation budget — one object across all ctx spreads. */
  budget: { ops: number }
  /** Shared mutable flag — set when preview quality actually altered a value. */
  reduced: { value: boolean }
  instancePath: string
  valueBudget: { used: number }
  valueWeights: WeakMap<Value[], number>
  valueDepths: WeakMap<Value[], number>
  /** Nested roots are ignored once an enclosing runtime root is selected. */
  viewportRootLocked?: boolean
  /** The selected call ignores its own presentation modifiers. */
  viewportRootOwner?: CallNode
  /**
   * Optional cooperative cancel/yield handle. Spread onto child contexts so
   * nested loops can poll; top-level `for` also awaits mid-iteration yields.
   */
  control?: CooperativeCheckpoint
}

interface PassedCallChildren {
  readonly statements: readonly Statement[]
  readonly env: ReadonlyMap<string, Value>
  readonly scope: OpenScadStableScope
  readonly continuation?: Statement[] | PassedCallChildren
}

interface StableFunctionValue extends FunctionValue {
  readonly lexicalScope: OpenScadStableScope
}

interface Shape2D { dimension: 2; geometry: CadKernelHandle; color: RGBA; entityId: SceneEntityId }
interface Shape3D { dimension: 3; geometry: CadKernelHandle; color: RGBA; entityId: SceneEntityId }
type Shape = Shape2D | Shape3D

class StableViewportRootSelection {
  constructor(readonly shapes: Shape[]) {}
}

function currentEntityId(ctx: EvalContext): SceneEntityId {
  return boundedSceneEntityId(ctx.instancePath)
}

function staticOperationId(node: CallNode): SourceOperationId {
  return node.operationId ?? `op:legacy-offset-${node.p}`
}

function trackSource(geometry: CadKernelHandle, node: CallNode, ctx: EvalContext) {
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

function trackedSolid(geometry: CadKernelHandle, color: RGBA, node: CallNode, ctx: EvalContext): Shape3D {
  // Eager products (hull/boolean) start without a source id. Promote those
  // results so later transforms keep provenance for the generating call.
  const id = ctx.kernel.originalId(geometry)
  const trackedGeometry = id === null || id < 0 ? ctx.kernel.asOriginal(geometry) : geometry
  trackSource(trackedGeometry, node, ctx)
  return { dimension: 3, geometry: trackedGeometry, color, entityId: currentEntityId(ctx) }
}

function warn(ctx: EvalContext, message: string) { if (!ctx.warnings.includes(message)) ctx.warnings.push(message) }
function evaluationError(ctx: EvalContext, p: number, message: string): never { throw new OpenSCADParseError(ctx.source, p, message) }
function kernelCall<T>(ctx: EvalContext, p: number, run: () => T): T {
  try {
    return run()
  } catch (error) {
    if (error instanceof OpenSCADParseError || error instanceof AbortedError) throw error
    throw positionKernelError(ctx.source, p, error)
  }
}

function valueWeight(value: Value, ctx: EvalContext): number {
  if (Array.isArray(value)) return ctx.valueWeights.get(value) ?? value.length + 1
  return typeof value === 'string' ? Math.max(1, value.length) : 1
}

function valueDepth(value: Value, ctx: EvalContext): number {
  return Array.isArray(value) ? ctx.valueDepths.get(value) ?? 1 : 0
}

function registerArrayValue<T extends Value[]>(
  value: T,
  ctx: EvalContext,
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

function registerStringValue(value: string, ctx: EvalContext, p: number, limitLabel = 'Evaluation'): string {
  ctx.valueBudget.used += Math.max(1, value.length)
  if (ctx.valueBudget.used > MAX_EVALUATED_VALUE_UNITS) {
    evaluationError(ctx, p, `${limitLabel} exceeds the ${MAX_EVALUATED_VALUE_UNITS.toLocaleString()} value-allocation budget`)
  }
  return value
}

class StableBuiltinValueError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'StableBuiltinValueError'
  }
}

function isStableProfile(ctx: EvalContext): boolean {
  return ctx.languageProfile === 'openscad/stable-2021.01'
}

function resolveStableVariable(name: string, ctx: EvalContext): { found: boolean; value: Value } {
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

function stableOverlayContext(ctx: EvalContext, env: ReadonlyMap<string, Value>): EvalContext {
  if (!isStableProfile(ctx)) return { ...ctx, env: new Map(env) }
  const scope = new OpenScadStableScope([], ctx.stableScope ?? null, env)
  return {
    ...ctx,
    env: scope.env,
    stableScope: scope,
    scopeVisibleBefore: Number.POSITIVE_INFINITY,
  }
}

function enterStableStatementScope(statements: readonly Statement[], parent: EvalContext): EvalContext {
  const scope = new OpenScadStableScope(statements, parent.stableScope ?? null, parent.env)
  const ctx: EvalContext = {
    ...parent,
    env: scope.env,
    stableScope: scope,
    scopeVisibleBefore: Number.POSITIVE_INFINITY,
  }
  for (const name of scope.dynamicVariableNames()) {
    const resolved = scope.resolveLocalVariable(
      name,
      Number.POSITIVE_INFINITY,
      (expression, site) => evalExpression(expression, {
        ...ctx,
        env: site.env,
        stableScope: site.scope,
        scopeVisibleBefore: site.visibleBefore,
      }),
      message => warn(ctx, message),
    )
    if (resolved.found) scope.env.set(name, resolved.value)
  }
  return ctx
}

function overlayDynamicVariables(target: Map<string, Value>, source: ReadonlyMap<string, Value>): void {
  for (const [name, value] of source) if (name.startsWith('$')) target.set(name, value)
}

function stableValueContext(ctx: EvalContext, position: number): OpenScadValueSemanticsContext {
  return {
    warn: message => warn(ctx, message),
    maxRangeItems: MAX_RANGE_ITEMS,
    registerArray: (values, label) => registerArrayValue(values, ctx, position, label),
  }
}

function isListComprehensionExpression(expr: Expr): expr is ListComprehensionExpression {
  return expr.kind === 'lc-for'
    || expr.kind === 'lc-for-c'
    || expr.kind === 'lc-if'
    || expr.kind === 'lc-let'
    || expr.kind === 'lc-each'
}

function evaluateSequentialBindings(
  args: readonly ExpressionArgument[],
  ctx: EvalContext,
  depth: number,
): Map<string, Value> {
  const env = new Map(ctx.env)
  const assigned = new Set<string>()
  for (const argument of args) {
    const value = evalExpression(argument.value, stableOverlayContext(ctx, env), depth + 1)
    if (argument.name === undefined) {
      warn(ctx, `Ignoring assignment without variable name ${formatOpenScadValue(value)}`)
      continue
    }
    if (assigned.has(argument.name)) {
      warn(ctx, `Ignoring duplicate variable assignment ${argument.name} = ${formatOpenScadValue(value)}`)
      continue
    }
    assigned.add(argument.name)
    env.set(argument.name, value)
  }
  return env
}

function stableIterable(value: Value, ctx: EvalContext, position: number): Value[] {
  if (isOpenScadRange(value)) return materializeOpenScadRange(value, stableValueContext(ctx, position))
  if (Array.isArray(value)) return value
  if (typeof value === 'string') {
    return registerArrayValue(Array.from(value), ctx, position, 'string iteration')
  }
  return value === undefined ? [] : [value]
}

function appendComprehensionValues(
  output: Value[],
  values: readonly Value[],
  expr: Expr,
  ctx: EvalContext,
): void {
  if (output.length + values.length > MAX_VALUE_ELEMENTS) {
    evaluationError(ctx, expr.p, `List comprehension exceeds ${MAX_VALUE_ELEMENTS.toLocaleString()} elements`)
  }
  output.push(...values)
}

function evalComprehensionElement(expr: Expr, ctx: EvalContext, depth: number): Value[] {
  const value = evalExpression(expr, ctx, depth + 1)
  return isListComprehensionExpression(expr) && Array.isArray(value) ? value : [value]
}

function evalListComprehension(
  expr: ListComprehensionExpression,
  ctx: EvalContext,
  depth: number,
): Value[] {
  const output: Value[] = []
  switch (expr.kind) {
    case 'lc-each':
      return stableIterable(evalExpression(expr.value, ctx, depth + 1), ctx, expr.p)
    case 'lc-if': {
      const selected = openScadTruthy(evalExpression(expr.condition, ctx, depth + 1))
        ? expr.yes
        : expr.no
      return selected === undefined ? output : evalComprehensionElement(selected, ctx, depth + 1)
    }
    case 'lc-let': {
      const env = evaluateSequentialBindings(expr.args, ctx, depth + 1)
      return evalListComprehension(expr.body, stableOverlayContext(ctx, env), depth + 1)
    }
    case 'lc-for': {
      const visit = (bindingIndex: number, iterationContext: EvalContext): void => {
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
          evalExpression(binding.value, iterationContext, depth + 1),
          iterationContext,
          binding.p,
        )
        if (binding.name === undefined) {
          warn(ctx, 'Ignoring for() iterator without variable name')
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
      let iterationContext: EvalContext = {
        ...ctx,
        env: evaluateSequentialBindings(expr.init, ctx, depth + 1),
      }
      iterationContext = stableOverlayContext(ctx, iterationContext.env)
      while (openScadTruthy(evalExpression(expr.condition, iterationContext, depth + 1))) {
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

function resolveStableExpressionArguments(
  args: readonly ExpressionArgument[],
  parameterNames: readonly string[],
  ctx: EvalContext,
): Map<string, ExpressionArgument> {
  const resolved = new Map<string, ExpressionArgument>()
  const parameters = new Set(parameterNames)
  for (const argument of args) {
    const name = argument.name ?? parameterNames.find(parameter => !resolved.has(parameter))
    if (name === undefined) {
      warn(ctx, 'Ignoring excess positional argument')
      continue
    }
    if (!parameters.has(name)) {
      warn(ctx, `Ignoring unknown argument ${name}`)
      continue
    }
    if (argument.name !== undefined && resolved.has(name)) {
      warn(ctx, `Argument ${name} was specified more than once`)
    }
    resolved.set(name, argument)
  }
  return resolved
}

/**
 * Bind a stable built-in module without re-evaluating argument expressions.
 * Positional slots and named-only special variables are deliberately separate:
 * `$fn` is valid by name but is never a seventh linear_extrude positional slot.
 */
function evaluateStableBuiltinModuleArguments(
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

interface EvaluatedStableCompatibilityArguments {
  readonly authored: readonly ExpressionArgument[]
  readonly values: ReadonlyMap<string, Value>
  readonly evaluated: ReadonlyMap<ExpressionArgument, Value>
}

function evaluateStableCompatibilityArguments(
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

function evalAssertExpression(
  expr: Extract<Expr, { kind: 'assert' }>,
  ctx: EvalContext,
  depth: number,
): Value {
  const resolved = resolveStableExpressionArguments(expr.args, ['condition', 'message'], ctx)
  const conditionArgument = resolved.get('condition')
  const messageArgument = resolved.get('message')
  const condition = conditionArgument
    ? evalExpression(conditionArgument.value, ctx, depth + 1)
    : undefined
  const message = messageArgument
    ? evalExpression(messageArgument.value, ctx, depth + 1)
    : undefined
  if (!openScadTruthy(condition)) {
    const conditionText = conditionArgument
      ? compactDiagnosticText(ctx.source.slice(conditionArgument.p, conditionArgument.end))
      : 'undef'
    const detail = messageArgument ? `: ${compactDiagnosticText(formatOpenScadValue(message))}` : ''
    evaluationError(ctx, expr.p, `Assertion '${conditionText}' failed${detail}`)
  }
  return expr.body === undefined ? undefined : evalExpression(expr.body, ctx, depth + 1)
}

function evalEchoExpression(
  expr: Extract<Expr, { kind: 'echo' }>,
  ctx: EvalContext,
  depth: number,
): Value {
  const values = expr.args.map(argument => {
    const value = formatOpenScadValue(evalExpression(argument.value, ctx, depth + 1))
    return argument.name === undefined ? value : `${argument.name} = ${value}`
  })
  ctx.warnings.push(`ECHO:${values.length ? ` ${values.join(', ')}` : ''}`)
  return expr.body === undefined ? undefined : evalExpression(expr.body, ctx, depth + 1)
}

function evalExpression(expr: Expr, ctx: EvalContext, depth = 0): Value {
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
    case 'binary': {
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
        const a = finiteNumber(left, ctx, expr.p, 'comparison operand')
        const b = finiteNumber(right, ctx, expr.p, 'comparison operand')
        if (expr.op === TT.Lt) return a < b
        if (expr.op === TT.Gt) return a > b
        if (expr.op === TT.LtEq) return a <= b
        return a >= b
      }
      const a = finiteNumber(left, ctx, expr.p, 'arithmetic operand')
      const b = finiteNumber(right, ctx, expr.p, 'arithmetic operand')
      let result: number
      if (expr.op === TT.Plus) result = a + b
      else if (expr.op === TT.Minus) result = a - b
      else if (expr.op === TT.Star) result = a * b
      else if (expr.op === TT.Slash) result = a / b
      else if (expr.op === TT.Percent) result = a % b
      else result = a ** b
      if (!Number.isFinite(result)) evaluationError(ctx, expr.p, 'Expression produced a non-finite number')
      return result
    }
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

function isFunctionValue(value: Value): value is FunctionValue {
  return !Array.isArray(value) && typeof value === 'object' && value !== null
    && value.kind === 'function-value'
}

function evalFunctionCall(expr: Extract<Expr, { kind: 'call' }>, ctx: EvalContext, depth: number): Value {
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
      ? resolveStableVariable(expr.name, ctx)
      : { found: ctx.env.has(expr.name), value: ctx.env.get(expr.name) }
    if (resolved.found && isFunctionValue(resolved.value)) {
      return invokeUserFunction(resolved.value, expr.args, ctx, depth)
    }
    return evalBuiltin(expr, ctx, depth)
  }
  const callee = evalExpression(expr.callee, ctx, depth + 1)
  if (!isFunctionValue(callee)) evaluationError(ctx, expr.p, 'Expression is not callable')
  return invokeUserFunction(callee, expr.args, ctx, depth)
}

function invokeUserFunction(
  fn: FunctionValue,
  args: readonly ExpressionArgument[],
  ctx: EvalContext,
  depth: number,
): Value {
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
      callerValues.set(argument, evalExpression(argument.value, ctx, depth + 1))
    }
    const definitionEnv = new Map(fn.closure)
    overlayDynamicVariables(definitionEnv, ctx.env)
    const definitionContext: EvalContext = {
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
        parameterValues.set(parameter.name, evalExpression(
          parameter.defaultValue,
          { ...definitionContext, env: new Map(definitionEnv) },
          depth + 1,
        ))
      } else parameterValues.set(parameter.name, undefined)
    }
    for (const [name, value] of parameterValues) env.set(name, value)
    return evalExpression(fn.body, {
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
  for (const argument of args) callerValues.set(argument, evalExpression(argument.value, ctx, depth + 1))
  const env = new Map(fn.closure)
  for (let index = 0; index < fn.params.length; index++) {
    const parameter = fn.params[index]
    const supplied = named.get(parameter.name) ?? positional[index]
    if (supplied) env.set(parameter.name, callerValues.get(supplied))
    else if (parameter.defaultValue) env.set(parameter.name, evalExpression(parameter.defaultValue, { ...ctx, env }, depth + 1))
    else env.set(parameter.name, undefined)
  }
  return evalExpression(fn.body, {
    ...ctx,
    env,
    functionStack: [...ctx.functionStack, fn.name ?? '<anonymous>'],
  }, depth + 1)
}

function compatibilityString(value: Value): string {
  if (value === undefined) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return formatOpenScadValue(value)
}

function evalDxfQueryBuiltin(
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

function evalBuiltin(expr: Extract<Expr, { kind: 'call' }>, ctx: EvalContext, depth: number): Value {
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

function valueToString(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(valueToString).join(', ')}]`
  if (isFunctionValue(value)) return 'function(...)'
  if (value === undefined) return 'undef'
  return String(value)
}

/** OpenSCAD boolean conversion: false, undef, zero and empty containers are false. */
function truthy(value: Value) {
  return value !== false
    && value !== undefined
    && value !== 0
    && value !== ''
    && (!Array.isArray(value) || value.length > 0)
}
function deepEqual(a: Value, b: Value): boolean {
  if (Array.isArray(a) && Array.isArray(b)) return a.length === b.length && a.every((value, index) => deepEqual(value, b[index]))
  return a === b
}
function finiteNumber(value: Value, ctx: EvalContext, p: number, label: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) evaluationError(ctx, p, `${label} must be a finite number`)
  return value
}
function vectorValue(value: Value, ctx: EvalContext, p: number, label: string): number[] {
  if (!Array.isArray(value)) evaluationError(ctx, p, `${label} must be a vector`)
  return value.map(item => finiteNumber(item, ctx, p, label))
}
function arg(node: CallNode, name: string, position: number, fallback: Value, ctx: EvalContext): Value {
  const expression = node.args[name] ?? node.args[`_${position}`]
  return expression ? evalExpression(expression, ctx) : fallback
}

function callExpressionArguments(node: CallNode): ExpressionArgument[] {
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

interface BoundAssertArguments {
  condition: Expr
  conditionText: string
  message?: Expr
}

function bindAssertArguments(node: CallNode, ctx: EvalContext): BoundAssertArguments {
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

function compactDiagnosticText(value: string, limit = 240): string {
  const compact = value.replace(/\s+/g, ' ').trim()
  return compact.length <= limit ? compact : `${compact.slice(0, limit - 1)}…`
}

function collectModules(nodes: readonly Statement[], modules: Map<string, ModuleNode>) {
  for (const node of nodes) {
    if (node.type === 'module') modules.set(node.name, node)
    if (node.type === 'call') {
      collectModules(node.children, modules)
      collectModules(node.alternative, modules)
    }
  }
}

function collectFunctions(nodes: readonly Statement[], functions: Map<string, FunctionNode>) {
  for (const node of nodes) {
    if (node.type === 'function') functions.set(node.name, node)
    if (node.type === 'call') {
      collectFunctions(node.children, functions)
      collectFunctions(node.alternative, functions)
    } else if (node.type === 'module') collectFunctions(node.children, functions)
  }
}

const STABLE_COMPATIBILITY_MODULE_FORMAT: Readonly<Partial<Record<string, OpenScad2021LegacyImportFormat>>> = Object.freeze({
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
function stableCompatibilityForcedAssets(
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

/** Yield to the event loop after this many statements or loop iterations… */
const YIELD_EVERY_STATEMENTS = 25
/** …or once this much wall-clock time has elapsed since the last yield. */
const YIELD_EVERY_MS = 50

async function evalNodes(nodes: readonly Statement[], parent: EvalContext, scoped = true): Promise<Shape[]> {
  const ctx: EvalContext = isStableProfile(parent)
    ? enterStableStatementScope(nodes, parent)
    : { ...parent, env: scoped ? new Map(parent.env) : parent.env }
  return await evalPreparedNodes(nodes, ctx)
}

function stableViewportDisabled(statement: Statement, ctx: EvalContext): boolean {
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
async function evalPreparedCall(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
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

async function evalPreparedNodes(nodes: readonly Statement[], ctx: EvalContext): Promise<Shape[]> {
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

class CooperativeCheckpoint {
  private lastYield: number
  private readonly enabled: boolean

  constructor(
    private readonly shouldAbort?: () => boolean,
    private readonly onYield?: () => void,
    private readonly now: () => number = () => performance.now(),
    private readonly yieldControl: () => Promise<void> = () => new Promise(resolve => setTimeout(resolve, 0)),
  ) {
    this.lastYield = now()
    this.enabled = shouldAbort !== undefined || onYield !== undefined
  }

  poll(): void {
    if (this.shouldAbort?.()) throw new AbortedError()
  }

  async yieldIfDue(force = false): Promise<void> {
    if (!this.enabled) return
    const current = this.now()
    if (!force && current - this.lastYield < YIELD_EVERY_MS) return
    await this.yieldControl()
    this.poll()
    this.onYield?.()
    this.lastYield = this.now()
  }
}

/**
 * Top-level evaluation with cooperative cancellation. Nested `for` / `if` /
 * module bodies share the same checkpoint, so a Worker can receive cancel
 * mid-loop. A single enormous kernel/BVH call still cannot yield mid-WASM;
 * the coordinator grace timer remains that hard boundary.
 */
async function evalTopLevel(nodes: readonly Statement[], ctx: EvalContext, control: CooperativeCheckpoint): Promise<Shape[]> {
  control.poll()
  try {
    return await evalPreparedNodes(nodes, { ...ctx, control })
  } catch (error) {
    if (error instanceof StableViewportRootSelection) return error.shapes
    throw error
  }
}

function viewportRootActivates(node: CallNode, shapes: readonly Shape[]): boolean {
  if (shapes.length > 0) return true
  // OpenSCAD does not create an empty CSG container for these transparent
  // effect/branch calls. Every other evaluated module instantiation can win a
  // root selection even when its resulting geometry is empty.
  return !['if', 'assert', 'echo', 'children', 'child'].includes(node.name)
}

function passedCallChildren(ctx: EvalContext): {
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

function compatibilityDeprecation(
  node: CallNode,
  ctx: EvalContext,
  replacement: string,
): void {
  const message = node.name === 'child'
    ? 'child() will be removed in future releases. Use children() instead.'
    : `The ${node.name}() module will be removed in future releases. Use ${replacement} instead.`
  warn(ctx, message)
}

async function evalNode(node: CallNode, parent: EvalContext): Promise<Shape[]> {
  if (isStableProfile(parent) && !parent.viewportRootLocked
    && hasOpenScadViewportModifier(node, 'root')) {
    const shapes = await evalNode(node, {
      ...parent,
      viewportRootLocked: true,
      viewportRootOwner: node,
    })
    if (viewportRootActivates(node, shapes)) throw new StableViewportRootSelection(shapes)
    return shapes
  }
  if (parent.depth >= MAX_EVAL_DEPTH) evaluationError(parent, node.p, `Evaluation exceeds ${MAX_EVAL_DEPTH} nested calls`)
  const operationId = staticOperationId(node)
  const ctx: EvalContext = {
    ...parent,
    depth: parent.depth + 1,
    instancePath: `${parent.instancePath}>${operationId}`,
  }
  const childShapes = async () => await evalNodes(node.children, ctx)

  switch (node.name) {
    case 'assign': {
      requireStableProfile(ctx, node)
      const env = new Map(ctx.env)
      for (const argument of callExpressionArguments(node)) {
        // Historical assign() ignores positional arguments without evaluating
        // them, and each named RHS sees the caller rather than earlier args.
        if (argument.name === undefined) continue
        env.set(argument.name, evalExpression(argument.value, ctx))
      }
      return booleanShapes(
        await evalNodes(node.children, stableOverlayContext(ctx, env), false),
        'union',
        ctx,
        node.p,
        'assign',
      )
    }
    case 'assert': {
      if (isStableProfile(ctx)) {
        const args = callExpressionArguments(node)
        const resolved = resolveStableExpressionArguments(args, ['condition', 'message'], ctx)
        const conditionArgument = resolved.get('condition')
        const messageArgument = resolved.get('message')
        const condition = conditionArgument === undefined
          ? undefined
          : evalExpression(conditionArgument.value, ctx)
        const message = messageArgument === undefined
          ? undefined
          : evalExpression(messageArgument.value, ctx)
        if (!openScadTruthy(condition)) {
          const conditionText = conditionArgument === undefined
            ? 'undef'
            : compactDiagnosticText(ctx.source.slice(conditionArgument.p, conditionArgument.end))
          const detail = messageArgument === undefined
            ? ''
            : `: ${compactDiagnosticText(formatOpenScadValue(message))}`
          evaluationError(ctx, node.p, `Assertion '${conditionText}' failed${detail}`)
        }
        return await childShapes()
      }
      const bound = bindAssertArguments(node, ctx)
      const condition = evalExpression(bound.condition, ctx)
      // OpenSCAD binds call arguments before executing the module. Evaluate a
      // supplied message even when the assertion passes, but never evaluate
      // child geometry when the condition fails.
      const message = bound.message ? evalExpression(bound.message, ctx) : undefined
      if (!truthy(condition)) {
        const detail = bound.message
          ? `: ${compactDiagnosticText(valueToString(message))}`
          : ''
        evaluationError(ctx, node.p, `Assertion '${bound.conditionText}' failed${detail}`)
      }
      return await childShapes()
    }
    case 'cube': {
      if (isStableProfile(ctx)) return makeStableCube(node, ctx)
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'cube size') : [finiteNumber(raw, ctx, node.p, 'cube size')]
      const dimensions: Vec3 = [size[0] ?? 1, size[1] ?? size[0] ?? 1, size[2] ?? size[0] ?? 1]
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Cube dimensions must be positive')
      const geometry = kernelCall(ctx, node.p, () => ctx.kernel.box(dimensions, arg(node, 'center', 1, false, ctx) === true))
      return [trackedSolid(geometry, nextColor(), node, ctx)]
    }
    case 'sphere': {
      if (isStableProfile(ctx)) return makeStableSphere(node, ctx)
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'sphere diameter') / 2
      const r = finiteNumber(radius, ctx, node.p, 'sphere radius')
      if (r <= 0) evaluationError(ctx, node.p, 'Sphere radius must be positive')
      return [trackedSolid(kernelCall(ctx, node.p, () => ctx.kernel.sphere(r, segments(node, ctx, 32, 4, r))), nextColor(), node, ctx)]
    }
    case 'cylinder': return isStableProfile(ctx) ? makeStableCylinder(node, ctx) : makeCylinder(node, ctx)
    case 'polyhedron': return isStableProfile(ctx) ? makeStablePolyhedron(node, ctx) : makePolyhedron(node, ctx)
    case 'import': {
      requireStableProfile(ctx, node)
      return makeImport(node, ctx)
    }
    case 'import_stl':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'import()')
      return makeImport(node, ctx, 'stl')
    case 'import_off':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'import()')
      return makeImport(node, ctx, 'off')
    case 'import_dxf':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'import()')
      return makeImport(node, ctx, 'dxf')
    case 'surface': {
      requireStableProfile(ctx, node)
      return makeSurface(node, ctx)
    }
    case 'text': {
      requireStableProfile(ctx, node)
      return makeText(node, ctx)
    }
    case 'square': {
      if (isStableProfile(ctx)) return makeStableSquare(node, ctx)
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'square size') : [finiteNumber(raw, ctx, node.p, 'square size')]
      const dimensions: Vec2 = [size[0] ?? 1, size[1] ?? size[0] ?? 1]
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Square dimensions must be positive')
      return [{
        dimension: 2,
        geometry: ctx.kernel.rectangle(dimensions, arg(node, 'center', 1, false, ctx) === true),
        color: nextColor(),
        entityId: currentEntityId(ctx),
      }]
    }
    case 'circle': {
      if (isStableProfile(ctx)) return makeStableCircle(node, ctx)
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'circle diameter') / 2
      const r = finiteNumber(radius, ctx, node.p, 'circle radius')
      if (r <= 0) evaluationError(ctx, node.p, 'Circle radius must be positive')
      return [{
        dimension: 2,
        geometry: ctx.kernel.circle(r, segments(node, ctx, 48, 3, r)),
        color: nextColor(),
        entityId: currentEntityId(ctx),
      }]
    }
    case 'polygon': return isStableProfile(ctx) ? makeStablePolygon(node, ctx) : makePolygon(node, ctx)
    case 'translate': {
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx)
      const vector = vectorValue(arg(node, 'v', 0, [0, 0, 0], ctx), ctx, node.p, 'translate vector')
      return (await childShapes()).map(shape => ({
        ...shape,
        geometry: ctx.kernel.translate(
          shape.geometry,
          shape.dimension === 3
            ? [vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]
            : [vector[0] ?? 0, vector[1] ?? 0],
        ),
      }))
    }
    case 'rotate': return isStableProfile(ctx) ? await transformStableChildren(node, ctx) : rotateShapes(await childShapes(), node, ctx)
    case 'scale': {
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx)
      const raw = arg(node, 'v', 0, [1, 1, 1], ctx)
      const values = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'scale vector') : [finiteNumber(raw, ctx, node.p, 'scale')]
      const sx = values[0] ?? 1, sy = values[1] ?? sx, sz = values[2] ?? sx
      if ([sx, sy, sz].some(value => value === 0)) evaluationError(ctx, node.p, 'Scale values cannot be zero')
      return (await childShapes()).map(shape => ({
        ...shape,
        geometry: ctx.kernel.scale(
          shape.geometry,
          shape.dimension === 3 ? [sx, sy, sz] : [sx, sy],
        ),
      }))
    }
    case 'resize': {
      requireStableProfile(ctx, node)
      return await resizeChildren(node, ctx)
    }
    case 'mirror': {
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx)
      const vector = vectorValue(arg(node, 'v', 0, [1, 0, 0], ctx), ctx, node.p, 'mirror normal')
      return (await childShapes()).map(shape => ({
        ...shape,
        geometry: ctx.kernel.mirror(
          shape.geometry,
          shape.dimension === 3
            ? [vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]
            : [vector[0] ?? 0, vector[1] ?? 0],
        ),
      }))
    }
    case 'multmatrix': return isStableProfile(ctx)
      ? await transformStableChildren(node, ctx)
      : transformByMatrix(await childShapes(), arg(node, 'm', 0, undefined, ctx), node, ctx)
    case 'color': {
      const colorValue = arg(node, 'c', 0, isStableProfile(ctx) ? undefined : [0.5, 0.5, 0.5], ctx)
      const alpha = arg(node, 'alpha', 1, undefined, ctx)
      const color = isStableProfile(ctx)
        ? [...resolveOpenScadColor(colorValue, alpha, stableModuleSemanticsContext(ctx)).rgba] as RGBA
        : parseLegacyColor(colorValue, ctx, node.p)
      if (!isStableProfile(ctx) && alpha !== undefined) {
        color[3] = clamp01(finiteNumber(alpha, ctx, node.p, 'color alpha'))
      }
      return (await childShapes()).map(shape => ({ ...shape, color: [...color] as RGBA }))
    }
    case 'union': return booleanShapes(await childShapes(), 'union', ctx, node.p)
    case 'difference': return await differenceChildren(node, ctx)
    case 'intersection': return booleanShapes(await childShapes(), 'intersection', ctx, node.p)
    case 'minkowski': {
      requireStableProfile(ctx, node)
      return minkowskiShapes(await childShapes(), node, ctx)
    }
    case 'hull': return hullShapes(await childShapes(), node, ctx)
    case 'linear_extrude': return await linearExtrude(node, ctx)
    case 'rotate_extrude': return await rotateExtrude(node, ctx)
    case 'dxf_linear_extrude':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'linear_extrude()')
      return await dxfLinearExtrude(node, ctx)
    case 'dxf_rotate_extrude':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'rotate_extrude()')
      return await dxfRotateExtrude(node, ctx)
    case 'projection': {
      if (isStableProfile(ctx)) {
        const values = evaluateStableBuiltinModuleArguments(
          node,
          ['cut', 'convexity'],
          [],
          ctx,
        )
        const cut = values.get('cut') === true
        const children = await childShapes()
        const solids = children.filter((shape): shape is Shape3D => shape.dimension === 3)
        if (solids.length !== children.length) {
          warn(ctx, 'projection() ignored non-3D child geometry')
        }
        if (solids.length === 0) return []
        const color = solids[0].color
        const geometry = cut
          ? ctx.kernel.projection(ctx.kernel.boolean3('union', solids.map(shape => shape.geometry)), true)
          : ctx.kernel.boolean2('union', solids.map(shape => ctx.kernel.projection(shape.geometry, false)))
        return [{ dimension: 2, geometry, color, entityId: currentEntityId(ctx) }]
      }
      const cut = arg(node, 'cut', 0, false, ctx) === true
      return (await childShapes()).map(shape => {
        if (shape.dimension !== 3) evaluationError(ctx, node.p, 'projection() requires 3D children')
        return {
          dimension: 2,
          geometry: ctx.kernel.projection(shape.geometry, cut),
          color: shape.color,
          entityId: currentEntityId(ctx),
        }
      })
    }
    case 'offset': {
      if (isStableProfile(ctx)) {
        const rExpression = node.args.r ?? node.args._0
        const deltaExpression = node.args.delta
        const chamferExpression = node.args.chamfer
        const resolved = resolveOpenScadOffset({
          ...(rExpression === undefined ? {} : { r: evalExpression(rExpression, ctx) }),
          ...(deltaExpression === undefined ? {} : { delta: evalExpression(deltaExpression, ctx) }),
          ...(chamferExpression === undefined ? {} : { chamfer: evalExpression(chamferExpression, ctx) }),
        }, stableModuleSemanticsContext(ctx))
        const circularSegments = resolved.joinType === 'Round'
          ? segments(node, ctx, 48, 3, Math.abs(resolved.distance))
          : undefined
        return (await childShapes()).map(shape => {
          if (shape.dimension !== 2) evaluationError(ctx, node.p, 'offset() requires 2D children')
          return {
            ...shape,
            geometry: ctx.kernel.offset(
              shape.geometry,
              resolved.distance,
              resolved.joinType,
              resolved.joinType === 'Miter' ? 1_000_000_000 : 2,
              circularSegments,
            ),
            entityId: currentEntityId(ctx),
          }
        })
      }
      const distance = finiteNumber(arg(node, 'r', 0, arg(node, 'delta', 0, 1, ctx), ctx), ctx, node.p, 'offset distance')
      return (await childShapes()).map(shape => {
        if (shape.dimension !== 2) evaluationError(ctx, node.p, 'offset() requires 2D children')
        return { ...shape, geometry: ctx.kernel.offset(shape.geometry, distance), entityId: currentEntityId(ctx) }
      })
    }
    case 'group': {
      const shapes = await childShapes()
      return isStableProfile(ctx) ? booleanShapes(shapes, 'union', ctx, node.p, 'group') : shapes
    }
    case 'render': {
      if (isStableProfile(ctx)) {
        evaluateStableBuiltinModuleArguments(node, ['convexity'], [], ctx)
        return booleanShapes(await childShapes(), 'union', ctx, node.p, 'render')
      }
      return await childShapes()
    }
    case 'if': {
      const condition = arg(node, '_0', 0, false, ctx)
      const branch = isStableProfile(ctx) ? openScadTruthy(condition) : truthy(condition)
      return await evalNodes(branch ? node.children : node.alternative, ctx)
    }
    case 'let': {
      if (isStableProfile(ctx)) {
        const env = evaluateSequentialBindings(callExpressionArguments(node), ctx, 0)
        return await evalNodes(node.children, stableOverlayContext(ctx, env), false)
      }
      const env = new Map(ctx.env)
      for (const [name, expression] of Object.entries(node.args)) if (!name.startsWith('_')) env.set(name, evalExpression(expression, ctx))
      return await evalNodes(node.children, { ...ctx, env }, false)
    }
    case 'for': return await evalFor(node, ctx, 'for')
    case 'intersection_for': {
      requireStableProfile(ctx, node)
      return booleanShapes(
        await evalFor(node, ctx, 'intersection_for'),
        'intersection',
        ctx,
        node.p,
        'intersection_for',
      )
    }
    case 'echo': {
      requireStableProfile(ctx, node)
      const values = Object.entries(node.args).map(([name, expression]) => {
        const value = formatOpenScadValue(evalExpression(expression, ctx))
        return node.argKinds[name] === 'named' ? `${name} = ${value}` : value
      })
      ctx.warnings.push(`ECHO:${values.length ? ` ${values.join(', ')}` : ''}`)
      return await childShapes()
    }
    case 'children': {
      const passed = passedCallChildren(ctx)
      const all = passed.statements
      const childContext = passed.context
      if (isStableProfile(ctx)) {
        const selectorExpression = node.args._0
        const selected = resolveOpenScadChildrenSelection({
          provided: selectorExpression !== undefined,
          value: selectorExpression === undefined ? undefined : evalExpression(selectorExpression, ctx),
          childCount: all.length,
          maximumRangeItems: MAX_RANGE_ITEMS,
        }, stableGeometrySemanticsContext(ctx))
        if (selectorExpression === undefined) return await evalNodes(all, childContext)
        const output: Shape[] = []
        for (const [occurrence, childIndex] of selected.entries()) {
          output.push(...await evalNodes([all[childIndex]], {
            ...childContext,
            instancePath: `${childContext.instancePath}>children:${childIndex}#${occurrence}`,
          }))
        }
        return output
      }
      const index = arg(node, '_0', 0, undefined, ctx)
      if (index === undefined) return await evalNodes(all, childContext)
      const childIndex = Math.trunc(finiteNumber(index, ctx, node.p, 'children index'))
      return all[childIndex] ? await evalNodes([all[childIndex]], childContext) : []
    }
    case 'child': {
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'children()')
      const authored = callExpressionArguments(node)
      const indexValue = authored[0] === undefined
        ? 0
        : evalExpression(authored[0].value, ctx)
      const index = typeof indexValue === 'number' && Number.isFinite(indexValue)
        ? Math.trunc(indexValue)
        : 0
      if (index < 0) {
        warn(ctx, `Negative child index (${index}) not allowed`)
        return []
      }
      const passed = passedCallChildren(ctx)
      const statement = passed.statements[index]
      if (statement === undefined) {
        if (ctx.callChildren !== undefined) {
          warn(ctx, `Child index (${index}) out of bounds (${passed.statements.length} children)`)
        }
        return []
      }
      return await evalNodes([statement], {
        ...passed.context,
        instancePath: `${passed.context.instancePath}>child:${index}`,
      })
    }
  }

  if (isStableProfile(ctx)) {
    const declaration = ctx.stableScope?.moduleDeclaration(node.name)
    if (declaration !== undefined) return await evalUserModule(node, declaration.node, ctx, declaration.scope)
  } else {
    const module = ctx.modules.get(node.name)
    if (module) return await evalUserModule(node, module, ctx)
  }
  evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
}

function requireStableProfile(ctx: EvalContext, node: CallNode): void {
  if (ctx.languageProfile !== 'openscad/stable-2021.01') {
    evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
  }
}

function stableModuleSemanticsContext(ctx: EvalContext): OpenScadStableModuleSemanticsContext {
  return { warn: warning => warn(ctx, warning.message) }
}

function stableGeometrySemanticsContext(ctx: EvalContext): OpenScadStableGeometryContext {
  return { warn: warning => warn(ctx, warning.message) }
}

function stablePrimitiveSemanticsContext(ctx: EvalContext): OpenScadStablePrimitiveContext {
  return {
    checkParameterRanges: true,
    warn: warning => warn(ctx, warning.message),
  }
}

function stableTransformSemanticsContext(ctx: EvalContext): OpenScadStableTransformContext {
  return {
    checkParameterRanges: true,
    warn: warning => warn(ctx, warning.message),
  }
}

function stableFragmentInput(
  node: CallNode,
  ctx: EvalContext,
  radius: number,
): OpenScadFragmentResolutionInput {
  const special = (name: '$fn' | '$fa' | '$fs'): { present: boolean; value: Value } => {
    const local = node.args[name]
    if (local !== undefined) return { present: true, value: evalExpression(local, ctx) }
    const resolved = resolveStableVariable(name, ctx)
    return { present: resolved.found, value: resolved.value }
  }
  const fn = special('$fn'), fa = special('$fa'), fs = special('$fs')
  return {
    radius,
    quality: ctx.quality,
    ...(fn.present ? { fn: fn.value } : {}),
    ...(fa.present ? { fa: fa.value } : {}),
    ...(fs.present ? { fs: fs.value } : {}),
  }
}

function stableFragmentInputFromEvaluated(
  evaluated: ReadonlyMap<string, Value>,
  ctx: EvalContext,
  radius: number,
): OpenScadFragmentResolutionInput {
  const special = (name: '$fn' | '$fa' | '$fs'): { present: boolean; value: Value } => {
    if (evaluated.has(name)) return { present: true, value: evaluated.get(name) }
    const resolved = resolveStableVariable(name, ctx)
    return { present: resolved.found, value: resolved.value }
  }
  const fn = special('$fn'), fa = special('$fa'), fs = special('$fs')
  return {
    radius,
    quality: ctx.quality,
    ...(fn.present ? { fn: fn.value } : {}),
    ...(fa.present ? { fa: fa.value } : {}),
    ...(fs.present ? { fs: fs.value } : {}),
  }
}

function stableSegmentsFromEvaluated(
  evaluated: ReadonlyMap<string, Value>,
  ctx: EvalContext,
  radius: number,
): number {
  const resolution = resolveOpenScadFragments(
    stableFragmentInputFromEvaluated(evaluated, ctx, radius),
    stableModuleSemanticsContext(ctx),
  )
  if (resolution.reduced) ctx.reduced.value = true
  return resolution.fragments
}

function stableEmptyShape(dimension: 2 | 3, color: RGBA, ctx: EvalContext): Shape {
  if (dimension === 3) {
    return {
      dimension: 3,
      geometry: ctx.kernel.empty3(),
      color,
      entityId: currentEntityId(ctx),
    }
  }
  return {
    dimension: 2,
    geometry: ctx.kernel.empty2(),
    color,
    entityId: currentEntityId(ctx),
  }
}

function warnIgnoredPrimitiveChildren(node: CallNode, ctx: EvalContext): void {
  if (node.children.length > 0) warn(ctx, `${node.name}() ignores child geometry`)
}

function makeStableCube(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(node, ['size', 'center'], [], ctx)
  const plan = resolveOpenScadCube({
    size: values.get('size'),
    center: values.get('center'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(3, color, ctx)]
  return [trackedSolid(
    ctx.kernel.box([...plan.dimensions] as Vec3, plan.center),
    color,
    node,
    ctx,
  )]
}

function makeStableSquare(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(node, ['size', 'center'], [], ctx)
  const plan = resolveOpenScadSquare({
    size: values.get('size'),
    center: values.get('center'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(2, color, ctx)]
  return [{
    dimension: 2,
    geometry: ctx.kernel.rectangle([...plan.dimensions] as Vec2, plan.center),
    color,
    entityId: currentEntityId(ctx),
  }]
}

function makeStableSphere(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['r'],
    ['d', '$fn', '$fa', '$fs'],
    ctx,
  )
  const plan = resolveOpenScadSphere({
    r: values.get('r'),
    d: values.get('d'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(3, color, ctx)]
  const fragments = stableSegmentsFromEvaluated(values, ctx, plan.fragmentRadius)
  return [trackedSolid(ctx.kernel.sphere(plan.radius, fragments), color, node, ctx)]
}

function makeStableCircle(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['r'],
    ['d', '$fn', '$fa', '$fs'],
    ctx,
  )
  const plan = resolveOpenScadCircle({
    r: values.get('r'),
    d: values.get('d'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(2, color, ctx)]
  const fragments = stableSegmentsFromEvaluated(values, ctx, plan.fragmentRadius)
  return [{
    dimension: 2,
    geometry: ctx.kernel.circle(plan.radius, fragments),
    color,
    entityId: currentEntityId(ctx),
  }]
}

function makeStableCylinder(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['h', 'r1', 'r2', 'center'],
    ['r', 'd', 'd1', 'd2', '$fn', '$fa', '$fs'],
    ctx,
  )
  const plan = resolveOpenScadCylinder({
    h: values.get('h'),
    r: values.get('r'),
    d: values.get('d'),
    r1: values.get('r1'),
    r2: values.get('r2'),
    d1: values.get('d1'),
    d2: values.get('d2'),
    center: values.get('center'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(3, color, ctx)]
  const fragments = stableSegmentsFromEvaluated(values, ctx, plan.fragmentRadius)
  let geometry: CadKernelHandle
  if (plan.radius1 > 0) {
    geometry = ctx.kernel.cylinder(
      plan.height,
      plan.radius1,
      plan.radius2,
      fragments,
      plan.center,
    )
  } else {
    geometry = ctx.kernel.mirrorZ(
      ctx.kernel.cylinder(plan.height, plan.radius2, 0, fragments, plan.center),
    )
    if (!plan.center) geometry = ctx.kernel.translateZ(geometry, plan.height)
  }
  return [trackedSolid(geometry, color, node, ctx)]
}

function makeStablePolyhedron(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['points', 'faces', 'convexity'],
    ['triangles'],
    ctx,
  )
  const plan = resolveOpenScadPolyhedron({
    points: values.get('points'),
    faces: values.get('faces'),
    triangles: values.get('triangles'),
    convexity: values.get('convexity'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(3, color, ctx)]

  const vertices: number[] = []
  const indices: number[] = []
  for (const polygon of plan.polygons) {
    if (polygon.length < 3) continue
    const base = vertices.length / 3
    for (const point of polygon) vertices.push(point[0], point[1], point[2])
    // OpenSCAD's authored exterior winding is opposite the kernel mesh input.
    for (let index = 1; index < polygon.length - 1; index++) {
      indices.push(base, base + index + 1, base + index)
    }
  }
  if (indices.length === 0 || vertices.some(value => !Number.isFinite(Math.fround(value)))) {
    warn(ctx, 'polyhedron() produced no usable finite faces')
    return [stableEmptyShape(3, color, ctx)]
  }

  try {
    const geometry = ctx.kernel.ofMesh(new Float32Array(vertices), new Uint32Array(indices))
    if (ctx.kernel.isEmpty(geometry)) {
      warn(ctx, 'polyhedron() topology did not produce a manifold solid')
      return [stableEmptyShape(3, color, ctx)]
    }
    return [trackedSolid(geometry, color, node, ctx)]
  } catch {
    warn(ctx, 'polyhedron() topology did not produce a manifold solid')
    return [stableEmptyShape(3, color, ctx)]
  }
}

function makeStablePolygon(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['points', 'paths', 'convexity'],
    [],
    ctx,
  )
  const plan = resolveOpenScadPolygon({
    points: values.get('points'),
    paths: values.get('paths'),
    convexity: values.get('convexity'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty || plan.outlines.some(outline =>
    outline.some(point => !Number.isFinite(point[0]) || !Number.isFinite(point[1])))) {
    if (!plan.empty) warn(ctx, 'polygon() produced no usable finite outlines')
    return [stableEmptyShape(2, color, ctx)]
  }

  try {
    const polygons = plan.outlines.map(outline =>
      outline.map(point => [point[0], point[1]] as Vec2))
    const geometry = ctx.kernel.polygon(polygons, 'EvenOdd')
    if (ctx.kernel.isEmpty(geometry)) warn(ctx, 'polygon() outlines produced an empty cross-section')
    return [{
      dimension: 2,
      geometry,
      color,
      entityId: currentEntityId(ctx),
    }]
  } catch {
    warn(ctx, 'polygon() outlines did not produce a valid cross-section')
    return [stableEmptyShape(2, color, ctx)]
  }
}

function segments(node: CallNode, ctx: EvalContext, fallback: number, minimum: number, radius: number): number {
  if (isStableProfile(ctx)) {
    const resolution = resolveOpenScadFragments(
      stableFragmentInput(node, ctx, radius),
      stableModuleSemanticsContext(ctx),
    )
    if (resolution.reduced) ctx.reduced.value = true
    return resolution.fragments
  }
  const local = arg(node, '$fn', -1, undefined, ctx)
  const global = ctx.env.get('$fn')
  const raw = local === undefined || local === 0 ? global : local
  const requested = raw === undefined || raw === 0 ? undefined : Math.round(finiteNumber(raw, ctx, node.p, '$fn'))
  const maxSegments = ctx.quality === 'preview' ? 48 : MAX_FN
  const previewFallback = ctx.quality === 'preview' ? Math.min(fallback, 24) : fallback
  let value = requested ?? previewFallback
  if (value > maxSegments) {
    warn(ctx, `$fn=${value} was clamped to ${maxSegments} for ${ctx.quality} rendering`)
    value = maxSegments
  }
  value = Math.max(minimum, value)
  if (ctx.quality === 'preview') {
    // This is the only place quality changes evaluation. Record whether the
    // preview reduction actually altered the segment count a full-quality
    // evaluation of the same call would have used.
    const fullValue = Math.max(minimum, Math.min(requested ?? fallback, MAX_FN))
    if (value !== fullValue) ctx.reduced.value = true
  }
  return value
}

function makeCylinder(node: CallNode, ctx: EvalContext): Shape[] {
  const height = finiteNumber(arg(node, 'h', 0, 1, ctx), ctx, node.p, 'cylinder height')
  if (height <= 0) evaluationError(ctx, node.p, 'Cylinder height must be positive')
  let low = arg(node, 'r1', 1, undefined, ctx)
  let high = arg(node, 'r2', 2, undefined, ctx)
  const radius = arg(node, 'r', 1, undefined, ctx)
  const diameter = arg(node, 'd', -1, undefined, ctx)
  const d1 = arg(node, 'd1', -1, undefined, ctx)
  const d2 = arg(node, 'd2', -1, undefined, ctx)
  if (d1 !== undefined) low = finiteNumber(d1, ctx, node.p, 'd1') / 2
  if (d2 !== undefined) high = finiteNumber(d2, ctx, node.p, 'd2') / 2
  if (low === undefined && high === undefined) {
    const base = diameter !== undefined ? finiteNumber(diameter, ctx, node.p, 'diameter') / 2
      : radius !== undefined ? finiteNumber(radius, ctx, node.p, 'radius') : 1
    low = high = base
  }
  if (low === undefined) low = high
  if (high === undefined) high = low
  const r1 = finiteNumber(low, ctx, node.p, 'r1')
  const r2 = finiteNumber(high, ctx, node.p, 'r2')
  if (r1 < 0 || r2 < 0 || (r1 === 0 && r2 === 0)) evaluationError(ctx, node.p, 'Cylinder radii must be non-negative and not both zero')
  const center = arg(node, 'center', 3, false, ctx) === true
  const fn = segments(node, ctx, 32, 3, Math.max(r1, r2))
  let geometry: CadKernelHandle
  if (r1 > 0) geometry = ctx.kernel.cylinder(height, r1, r2, fn, center)
  else {
    geometry = ctx.kernel.mirrorZ(ctx.kernel.cylinder(height, r2, 0, fn, center))
    if (!center) geometry = ctx.kernel.translateZ(geometry, height)
  }
  return [trackedSolid(geometry, nextColor(), node, ctx)]
}

function makePolyhedron(node: CallNode, ctx: EvalContext): Shape[] {
  const points = arg(node, 'points', 0, [], ctx)
  const faces = arg(node, 'faces', 1, arg(node, 'triangles', 1, [], ctx), ctx)
  if (!Array.isArray(points) || !Array.isArray(faces)) evaluationError(ctx, node.p, 'polyhedron points and faces must be vectors')
  const vertices: number[] = []
  for (const point of points) {
    const vector = vectorValue(point, ctx, node.p, 'polyhedron point')
    if (vector.length < 3) evaluationError(ctx, node.p, 'Each polyhedron point needs three coordinates')
    vertices.push(vector[0], vector[1], vector[2])
  }
  const indices: number[] = []
  for (const face of faces) {
    const polygon = vectorValue(face, ctx, node.p, 'polyhedron face').map(Math.trunc)
    if (polygon.length < 3) evaluationError(ctx, node.p, 'Each polyhedron face needs at least three vertices')
    for (const index of polygon) if (index < 0 || index >= points.length) evaluationError(ctx, node.p, 'Polyhedron face index is out of bounds')
    // OpenSCAD faces are wound clockwise viewed from outside; the kernel
    // requires counter-clockwise — reverse the fan so spec-correct
    // polyhedra build outward-facing instead of inside-out.
    for (let i = 1; i < polygon.length - 1; i++) indices.push(polygon[0], polygon[i + 1], polygon[i])
  }
  try {
    // Weld duplicated coordinates first: OpenSCAD accepts point lists with
    // repeated positions, but kernel halfedge pairing rejects them.
    return [trackedSolid(ctx.kernel.ofMesh(new Float32Array(vertices), new Uint32Array(indices)), nextColor(), node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `Invalid manifold polyhedron: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function surfaceCallSource(node: CallNode, ctx: EvalContext): {
  readonly source: string
  readonly sourcePath: string
} {
  const sourcePath = node.sourcePath ?? ctx.project?.entrypoint ?? '<inline>'
  const file = ctx.project?.read(sourcePath)
  return {
    source: file?.kind === 'source' ? file.source : ctx.source,
    sourcePath,
  }
}

const TEXT_POSITIONAL_PARAMETERS = Object.freeze([
  'text',
  'size',
  'font',
] as const)

const TEXT_NAMED_PARAMETERS = new Set([
  ...TEXT_POSITIONAL_PARAMETERS,
  'direction',
  'language',
  'script',
  'halign',
  'valign',
  'spacing',
  '$fn',
  '$fa',
  '$fs',
])

function textArgumentValues(node: CallNode, ctx: EvalContext): Map<string, Value> {
  const resolved = new Map<string, ExpressionArgument>()
  for (const argument of callExpressionArguments(node)) {
    if (argument.name === undefined) {
      const name = TEXT_POSITIONAL_PARAMETERS.find(parameter => !resolved.has(parameter))
      if (name === undefined) {
        warn(ctx, 'Ignoring excess positional argument')
        continue
      }
      resolved.set(name, argument)
      continue
    }
    if (!TEXT_NAMED_PARAMETERS.has(argument.name)) {
      warn(ctx, `Ignoring unknown argument ${argument.name}`)
      continue
    }
    if (resolved.has(argument.name)) warn(ctx, `Argument ${argument.name} was specified more than once`)
    resolved.set(argument.name, argument)
  }

  const values = new Map<string, Value>()
  for (const [name, argument] of resolved) values.set(name, evalExpression(argument.value, ctx))
  return values
}

function textCallError(
  node: CallNode,
  ctx: EvalContext,
  code: OpenScadTextErrorCode,
  message: string,
  details: Omit<ConstructorParameters<typeof OpenScadTextPositionedError>[4], 'sourcePath'> = {},
): never {
  const authored = surfaceCallSource(node, ctx)
  throw new OpenScadTextPositionedError(authored.source, node, code, message, {
    sourcePath: authored.sourcePath,
    ...details,
  })
}

function textStringParameter(
  values: ReadonlyMap<string, Value>,
  name: string,
  fallback: string,
  node: CallNode,
  ctx: EvalContext,
): string {
  const value = values.get(name) ?? fallback
  if (typeof value !== 'string') {
    return textCallError(node, ctx, 'E_TEXT_PARAMETER_INVALID', `text() ${name} must be a string.`)
  }
  return value
}

function textNumberParameter(
  value: Value,
  name: string,
  fallback: number,
  node: CallNode,
  ctx: EvalContext,
): number {
  const selected = value ?? fallback
  if (typeof selected !== 'number' || !Number.isFinite(selected)) {
    return textCallError(node, ctx, 'E_TEXT_PARAMETER_INVALID', `text() ${name} must be a finite number.`)
  }
  return selected
}

function makeText(node: CallNode, ctx: EvalContext): Shape[] {
  if (ctx.project === undefined || ctx.textAssets === undefined) {
    return textCallError(
      node,
      ctx,
      'E_TEXT_PROJECT_REQUIRED',
      'text() requires an OpenSCAD project so fonts are resolved inside the bounded project VFS.',
    )
  }

  const authored = surfaceCallSource(node, ctx)
  const argumentContext = authored.source === ctx.source
    ? ctx
    : { ...ctx, source: authored.source }
  const values = textArgumentValues(node, argumentContext)
  const size = textNumberParameter(values.get('size'), 'size', 10, node, argumentContext)
  const textSpecial = (name: '$fn' | '$fa' | '$fs'): { present: boolean; value: Value } => {
    if (values.has(name)) return { present: true, value: values.get(name) }
    const inherited = resolveStableVariable(name, argumentContext)
    return { present: inherited.found, value: inherited.value }
  }
  const textFn = textSpecial('$fn'), textFa = textSpecial('$fa'), textFs = textSpecial('$fs')
  const textFragmentInput: OpenScadFragmentResolutionInput = {
    radius: Math.max(0, size),
    quality: argumentContext.quality,
    ...(textFn.present ? { fn: textFn.value } : {}),
    ...(textFa.present ? { fa: textFa.value } : {}),
    ...(textFs.present ? { fs: textFs.value } : {}),
  }
  const textFragments = resolveOpenScadFragments(
    textFragmentInput,
    stableModuleSemanticsContext(argumentContext),
  )
  if (textFragments.reduced) argumentContext.reduced.value = true
  const parameters: OpenScadTextParameters = {
    text: textStringParameter(values, 'text', '', node, argumentContext),
    size,
    font: textStringParameter(values, 'font', '', node, argumentContext),
    direction: textStringParameter(values, 'direction', '', node, argumentContext),
    language: textStringParameter(values, 'language', 'en', node, argumentContext),
    script: textStringParameter(values, 'script', '', node, argumentContext),
    halign: textStringParameter(values, 'halign', 'left', node, argumentContext),
    valign: textStringParameter(values, 'valign', 'baseline', node, argumentContext),
    spacing: textNumberParameter(values.get('spacing'), 'spacing', 1, node, argumentContext),
    // Passing the bounded integer as explicit $fn makes the text flattener a
    // consumer of the same 2021 fragment decision as geometric primitives.
    $fn: textFragments.fragments,
    $fa: textFragments.effectiveFa,
    $fs: textFragments.effectiveFs,
  }

  try {
    const layout = renderOpenScadText(
      ctx.project,
      ctx.textAssets,
      parameters,
      authored.sourcePath,
    )
    const sections: CadKernelHandle[] = []
    for (const glyph of layout.glyphs) {
      if (glyph.contours.length === 0) continue
      const section = ctx.kernel.polygon(
        glyph.contours.map(contour => contour.map(point => [point[0], point[1]] as Vec2)),
        'EvenOdd',
      )
      if (!ctx.kernel.isEmpty(section)) sections.push(section)
    }
    if (sections.length === 0) return []
    const geometry = sections.length === 1
      ? sections[0]
      : ctx.kernel.boolean2('union', sections)
    if (ctx.kernel.isEmpty(geometry)) return []
    return [{
      dimension: 2,
      geometry,
      color: nextColor(),
      entityId: currentEntityId(ctx),
    }]
  } catch (error) {
    if (error instanceof OpenScadTextPositionedError) throw error
    if (error instanceof OpenScadTextError) {
      const { sourcePath: _sourcePath, ...details } = error.details
      return textCallError(node, ctx, error.code, error.message, details)
    }
    return textCallError(
      node,
      ctx,
      'E_TEXT_SHAPING_FAILED',
      `text() could not construct valid 2D glyph geometry: ${error instanceof Error ? error.message : String(error)}`,
      { font: parameters.font },
    )
  }
}

const IMPORT_POSITIONAL_PARAMETERS = Object.freeze([
  'file',
  'layer',
  'convexity',
  'origin',
  'scale',
] as const)

const IMPORT_NAMED_PARAMETERS = new Set([
  ...IMPORT_POSITIONAL_PARAMETERS,
  'width',
  'height',
  'center',
  'dpi',
  '$fn',
  '$fa',
  '$fs',
])

function importArgumentValues(node: CallNode, ctx: EvalContext): Map<string, Value> {
  const resolved = new Map<string, ExpressionArgument>()
  for (const argument of callExpressionArguments(node)) {
    if (argument.name === undefined) {
      const name = IMPORT_POSITIONAL_PARAMETERS.find(parameter => !resolved.has(parameter))
      if (name === undefined) {
        warn(ctx, 'Ignoring excess positional argument')
        continue
      }
      resolved.set(name, argument)
      continue
    }
    if (!IMPORT_NAMED_PARAMETERS.has(argument.name)) {
      warn(ctx, `Ignoring unknown argument ${argument.name}`)
      continue
    }
    if (resolved.has(argument.name)) warn(ctx, `Argument ${argument.name} was specified more than once`)
    resolved.set(argument.name, argument)
  }

  const values = new Map<string, Value>()
  for (const [name, argument] of resolved) values.set(name, evalExpression(argument.value, ctx))
  return values
}

function importCallError(
  node: CallNode,
  ctx: EvalContext,
  code: OpenScadImportErrorCode,
  message: string,
  details: {
    readonly specifier?: string
    readonly assetPath?: string
    readonly format?: ConstructorParameters<typeof OpenScadImportPositionedError>[4]['format']
    readonly limit?: number
    readonly actual?: number
  } = {},
): never {
  const authored = surfaceCallSource(node, ctx)
  throw new OpenScadImportPositionedError(authored.source, node, code, message, {
    sourcePath: authored.sourcePath,
    specifier: details.specifier ?? '',
    ...(details.assetPath === undefined ? {} : { assetPath: details.assetPath }),
    ...(details.format === undefined ? {} : { format: details.format }),
    ...(details.limit === undefined ? {} : { limit: details.limit }),
    ...(details.actual === undefined ? {} : { actual: details.actual }),
  })
}

function makeImport(
  node: CallNode,
  ctx: EvalContext,
  forcedFormat?: OpenScad2021LegacyImportFormat,
): Shape[] {
  const displayName = node.name
  if (ctx.project === undefined || ctx.importAssets === undefined) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_PROJECT_REQUIRED',
      `${displayName}() requires an OpenSCAD project so its file is resolved inside the bounded project VFS.`,
    )
  }

  const authored = surfaceCallSource(node, ctx)
  const argumentContext = authored.source === ctx.source
    ? ctx
    : { ...ctx, source: authored.source }
  const compatibility = forcedFormat !== undefined
  const values = compatibility
    ? evaluateStableBuiltinModuleArguments(
        node,
        IMPORT_POSITIONAL_PARAMETERS,
        ['width', 'height', 'filename', 'layername', 'center', 'dpi', '$fn', '$fa', '$fs'],
        argumentContext,
      )
    : importArgumentValues(node, argumentContext)
  const rawFileValue = values.get('file') ?? values.get('filename')
  const fileValue = compatibility ? compatibilityString(rawFileValue) : rawFileValue
  if (typeof fileValue !== 'string' || fileValue.length === 0) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_FILE_REQUIRED',
      `${displayName}() requires a non-empty string file path.`,
    )
  }

  const rawLayerValue = values.get('layer') ?? values.get('layername')
  const layerValue = compatibility && rawLayerValue !== undefined
    ? compatibilityString(rawLayerValue)
    : rawLayerValue
  if (layerValue !== undefined && typeof layerValue !== 'string') {
    return importCallError(node, ctx, 'E_IMPORT_ARGUMENT_INVALID', `${displayName}() layer must be a string.`, {
      specifier: fileValue,
    })
  }
  const originValue = values.get('origin')
  let origin: [number, number] | undefined
  if (originValue !== undefined) {
    if (!Array.isArray(originValue) || originValue.length !== 2
      || originValue.some(value => typeof value !== 'number' || !Number.isFinite(value))) {
      if (compatibility) {
        warn(ctx, `linear_extrude(..., origin=${formatOpenScadValue(originValue)}) could not be converted`)
      } else {
      return importCallError(
        node,
        ctx,
        'E_IMPORT_ARGUMENT_INVALID',
        `${displayName}() origin must contain two finite numbers.`,
        { specifier: fileValue },
      )
      }
    } else {
      origin = [originValue[0] as number, originValue[1] as number]
    }
  }
  const rawScaleValue = values.get('scale')
  const scaleValue = compatibility
    ? (typeof rawScaleValue === 'number' && Number.isFinite(rawScaleValue) && rawScaleValue > 0
        ? rawScaleValue
        : 1)
    : rawScaleValue
  if (!compatibility && scaleValue !== undefined
    && (typeof scaleValue !== 'number' || !Number.isFinite(scaleValue) || scaleValue <= 0)) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_ARGUMENT_INVALID',
      `${displayName}() scale must be a positive finite number.`,
      { specifier: fileValue },
    )
  }
  const rawCenterValue = values.get('center')
  const centerValue = compatibility ? rawCenterValue === true : rawCenterValue
  if (!compatibility && centerValue !== undefined && typeof centerValue !== 'boolean') {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_ARGUMENT_INVALID',
      `${displayName}() center must be boolean.`,
      { specifier: fileValue },
    )
  }
  const rawDpiValue = values.get('dpi')
  const dpiValue = compatibility
    ? (typeof rawDpiValue === 'number' && Number.isFinite(rawDpiValue) && rawDpiValue >= 0.001
        ? rawDpiValue
        : undefined)
    : rawDpiValue
  if (compatibility && typeof rawDpiValue === 'number'
    && (!Number.isFinite(rawDpiValue) || rawDpiValue < 0.001)) {
    warn(ctx, 'Invalid dpi value given; using the default 72 dpi')
  }
  if (!compatibility && dpiValue !== undefined
    && (typeof dpiValue !== 'number' || !Number.isFinite(dpiValue) || dpiValue < 0.001)) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_ARGUMENT_INVALID',
      `${displayName}() dpi must be a finite number greater than or equal to 0.001.`,
      { specifier: fileValue },
    )
  }

  // These 2021.01 parameters are evaluated even though they only affect the
  // legacy preview or are inert in the corresponding upstream importer.
  values.get('convexity')
  values.get('width')
  values.get('height')
  values.get('$fn')
  values.get('$fa')
  values.get('$fs')

  const importer = node.sourcePath ?? ctx.project.entrypoint
  const loadedScale = typeof scaleValue === 'number' ? scaleValue : undefined
  const loadedCenter = typeof centerValue === 'boolean' ? centerValue : undefined
  const loadedDpi = typeof dpiValue === 'number' ? dpiValue : undefined
  let assetPath: string | undefined
  try {
    const loaded = loadPreparedOpenScadImport(
      ctx.project,
      ctx.importAssets,
      importer,
      fileValue,
      {
        ...(layerValue === undefined ? {} : { layer: layerValue }),
        ...(origin === undefined ? {} : { origin }),
        ...(loadedScale === undefined ? {} : { scale: loadedScale }),
        ...(loadedCenter === undefined ? {} : { center: loadedCenter }),
        ...(loadedDpi === undefined ? {} : { dpi: loadedDpi }),
        ...(forcedFormat === undefined ? {} : { forcedFormat }),
      },
    )
    assetPath = resolveOpenScadProjectPath(importer, fileValue)

    if (loaded.dimension === 2) {
      const sections = loaded.regions.map(region => ctx.kernel.polygon(
        region.contours.map(contour => contour.map(point => [point[0], point[1]] as Vec2)),
        region.fillRule === 'evenodd' ? 'EvenOdd' : 'NonZero',
      ))
      const geometry = sections.length === 1
        ? sections[0]
        : ctx.kernel.boolean2('union', sections)
      if (ctx.kernel.isEmpty(geometry)) {
        return importCallError(
          node,
          ctx,
          'E_IMPORT_EMPTY',
          `${displayName}() produced empty 2D geometry.`,
          { specifier: fileValue, assetPath, format: loaded.format },
        )
      }
      return [{
        dimension: 2,
        geometry,
        color: nextColor(),
        entityId: currentEntityId(ctx),
      }]
    }

    const geometry = ctx.kernel.ofMesh(loaded.vertices, loaded.triangles)
    if (ctx.kernel.isEmpty(geometry)) {
      return importCallError(
        node,
        ctx,
        'E_IMPORT_EMPTY',
        `${displayName}() produced empty 3D geometry.`,
        { specifier: fileValue, assetPath, format: loaded.format },
      )
    }
    return [trackedSolid(geometry, nextColor(), node, ctx)]
  } catch (error) {
    if (error instanceof OpenScadImportPositionedError) throw error
    if (error instanceof OpenScadImportError) {
      return importCallError(node, ctx, error.code, error.message, {
        specifier: error.specifier,
        ...(error.assetPath === undefined ? {} : { assetPath: error.assetPath }),
        ...(error.format === undefined ? {} : { format: error.format }),
        ...(error.limit === undefined ? {} : { limit: error.limit }),
        ...(error.actual === undefined ? {} : { actual: error.actual }),
      })
    }
    return importCallError(
      node,
      ctx,
      'E_IMPORT_NON_MANIFOLD',
      `${displayName}() could not construct geometry: ${error instanceof Error ? error.message : String(error)}`,
      { specifier: fileValue, ...(assetPath === undefined ? {} : { assetPath }) },
    )
  }
}

function surfaceError(
  node: CallNode,
  ctx: EvalContext,
  code: ConstructorParameters<typeof OpenScadSurfaceError>[2],
  message: string,
  details: Omit<ConstructorParameters<typeof OpenScadSurfaceError>[4], 'sourcePath'> = {},
): never {
  const authored = surfaceCallSource(node, ctx)
  throw new OpenScadSurfaceError(authored.source, node, code, message, {
    sourcePath: authored.sourcePath,
    ...details,
  })
}

function makeSurface(node: CallNode, ctx: EvalContext): Shape[] {
  if (ctx.project === undefined || ctx.surfaceAssets === undefined) {
    return surfaceError(
      node,
      ctx,
      'E_SURFACE_PROJECT_REQUIRED',
      'surface() requires an OpenSCAD project so its file is resolved inside the bounded project VFS.',
    )
  }

  const authored = surfaceCallSource(node, ctx)
  const argumentContext = authored.source === ctx.source
    ? ctx
    : { ...ctx, source: authored.source }
  const fileValue = arg(node, 'file', 0, undefined, argumentContext)
  if (typeof fileValue !== 'string' || fileValue.length === 0) {
    return surfaceError(
      node,
      ctx,
      'E_SURFACE_FILE_REQUIRED',
      'surface() requires a non-empty string file path.',
    )
  }
  const center = arg(node, 'center', 1, false, argumentContext) === true
  // OpenSCAD 2021 stores convexity as a display/preview hint; evaluating the
  // argument preserves side effects and errors, but it cannot change the mesh.
  arg(node, 'convexity', 2, 1, argumentContext)
  const invert = node.args.invert === undefined
    ? false
    : evalExpression(node.args.invert, argumentContext) === true
  const importer = node.sourcePath ?? ctx.project.entrypoint

  let assetPath: string | undefined
  try {
    const loaded = loadOpenScadSurfaceHeightMap(
      ctx.project,
      ctx.surfaceAssets,
      importer,
      fileValue,
      invert,
    )
    assetPath = loaded.path
    const surface = triangulateOpenScadSurface(loaded.map, center)
    const geometry = ctx.kernel.ofMesh(surface.vertices, surface.triangles)
    return [trackedSolid(geometry, nextColor(), node, ctx)]
  } catch (error) {
    if (error instanceof OpenScadSurfaceError) throw error
    if (error instanceof OpenScadProjectError) {
      return surfaceError(
        node,
        ctx,
        'E_SURFACE_PATH_INVALID',
        error.message,
        {
          specifier: fileValue,
          ...(error.details.path === undefined ? {} : { assetPath: error.details.path }),
          ...(error.details.limit === undefined ? {} : { limit: error.details.limit }),
          ...(error.details.actual === undefined ? {} : { actual: error.details.actual }),
        },
      )
    }
    if (error instanceof OpenScadSurfaceDataError) {
      return surfaceError(
        node,
        ctx,
        error.code,
        error.message,
        {
          specifier: fileValue,
          ...(error.assetPath === undefined && assetPath === undefined
            ? {}
            : { assetPath: error.assetPath ?? assetPath }),
          ...(error.limit === undefined ? {} : { limit: error.limit }),
          ...(error.actual === undefined ? {} : { actual: error.actual }),
        },
      )
    }
    return surfaceError(
      node,
      ctx,
      'E_SURFACE_NON_MANIFOLD',
      `surface() could not construct manifold geometry: ${error instanceof Error ? error.message : String(error)}`,
      { specifier: fileValue, ...(assetPath === undefined ? {} : { assetPath }) },
    )
  }
}

function makePolygon(node: CallNode, ctx: EvalContext): Shape[] {
  const pointsValue = arg(node, 'points', 0, [], ctx)
  if (!Array.isArray(pointsValue)) evaluationError(ctx, node.p, 'polygon points must be a vector')
  const points = pointsValue.map(point => {
    const vector = vectorValue(point, ctx, node.p, 'polygon point')
    if (vector.length < 2) evaluationError(ctx, node.p, 'Each polygon point needs two coordinates')
    return [vector[0], vector[1]] as Vec2
  })
  const pathsValue = arg(node, 'paths', 1, undefined, ctx)
  let polygons: Vec2[][] = [points]
  if (pathsValue !== undefined) {
    if (!Array.isArray(pathsValue)) evaluationError(ctx, node.p, 'polygon paths must be a vector')
    polygons = pathsValue.map(path => vectorValue(path, ctx, node.p, 'polygon path').map(index => {
      const point = points[Math.trunc(index)]
      if (!point) evaluationError(ctx, node.p, 'Polygon path index is out of bounds')
      return point
    }))
  }
  // EvenOdd: the default Positive fill rule silently yields an EMPTY shape
  // for clockwise-wound point lists, which are perfectly valid in OpenSCAD.
  return [{
    dimension: 2,
    geometry: ctx.kernel.polygon(polygons, 'EvenOdd'),
    color: nextColor(),
    entityId: currentEntityId(ctx),
  }]
}

function stableTransformPlan(node: CallNode, ctx: EvalContext): OpenScadStableTransformPlan {
  const context = stableTransformSemanticsContext(ctx)
  switch (node.name) {
    case 'translate': {
      const values = evaluateStableBuiltinModuleArguments(node, ['v'], [], ctx)
      return resolveOpenScadTranslate(values.get('v'), context)
    }
    case 'scale': {
      const values = evaluateStableBuiltinModuleArguments(node, ['v'], [], ctx)
      return resolveOpenScadScale(values.get('v'), context)
    }
    case 'mirror': {
      const values = evaluateStableBuiltinModuleArguments(node, ['v'], [], ctx)
      return resolveOpenScadMirror(values.get('v'), context)
    }
    case 'rotate': {
      const values = evaluateStableBuiltinModuleArguments(node, ['a', 'v'], [], ctx)
      return resolveOpenScadRotate({ a: values.get('a'), v: values.get('v') }, context)
    }
    case 'multmatrix': {
      const values = evaluateStableBuiltinModuleArguments(node, ['m'], [], ctx)
      return resolveOpenScadMultmatrix(values.get('m'), context)
    }
    default:
      throw new Error(`Internal stable transform dispatch failed for ${node.name}()`)
  }
}

async function transformStableChildren(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  // Argument binding and expression effects happen before child instantiation.
  const plan = stableTransformPlan(node, ctx)
  const shapes = booleanShapes(
    await evalNodes(node.children, ctx),
    'union',
    ctx,
    node.p,
    node.name,
  )
  if (shapes.length === 0) return []

  const shape = shapes[0]
  const singular = shape.dimension === 3 ? plan.matrix3dSingular : plan.matrix2dSingular
  if (plan.dropsChildren || singular) {
    if (singular) warn(ctx, `${node.name}() produced empty geometry from a singular transform`)
    return [stableEmptyShape(shape.dimension, shape.color, ctx)]
  }

  try {
    if (shape.dimension === 3) {
      return [{
        ...shape,
        geometry: ctx.kernel.transform3(shape.geometry, [...plan.matrix]),
        entityId: currentEntityId(ctx),
      }]
    }
    return [{
      ...shape,
      geometry: ctx.kernel.transform2(shape.geometry, [...plan.matrix2d]),
      entityId: currentEntityId(ctx),
    }]
  } catch {
    warn(ctx, `${node.name}() transform could not be represented by the geometry kernel`)
    return [stableEmptyShape(shape.dimension, shape.color, ctx)]
  }
}

function rotateShapes(shapes: Shape[], node: CallNode, ctx: EvalContext): Shape[] {
  const angle = arg(node, 'a', 0, 0, ctx)
  const axis = arg(node, 'v', 1, undefined, ctx)
  return shapes.map(shape => {
    if (shape.dimension === 2) {
      const degrees = Array.isArray(angle) ? vectorValue(angle, ctx, node.p, 'rotation')[2] ?? 0 : finiteNumber(angle, ctx, node.p, 'rotation')
      return { ...shape, geometry: ctx.kernel.rotate(shape.geometry, degrees) }
    }
    if (Array.isArray(angle)) {
      const vector = vectorValue(angle, ctx, node.p, 'rotation')
      return { ...shape, geometry: ctx.kernel.rotate(shape.geometry, [vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]) }
    }
    const degrees = finiteNumber(angle, ctx, node.p, 'rotation')
    if (axis === undefined) return { ...shape, geometry: ctx.kernel.rotate(shape.geometry, [0, 0, degrees]) }
    const vector = vectorValue(axis, ctx, node.p, 'rotation axis')
    return { ...shape, geometry: ctx.kernel.transform3(shape.geometry, axisAngleMatrix(vector, degrees, ctx, node.p)) }
  })
}

function axisAngleMatrix(axis: number[], degrees: number, ctx: EvalContext, p: number): number[] {
  const length = Math.hypot(axis[0] ?? 0, axis[1] ?? 0, axis[2] ?? 0)
  if (length === 0) evaluationError(ctx, p, 'Rotation axis cannot be zero')
  const x = (axis[0] ?? 0) / length, y = (axis[1] ?? 0) / length, z = (axis[2] ?? 0) / length
  const angle = degrees * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), t = 1 - c
  // Kernel matrices are column-major.
  return [
    t*x*x+c, t*x*y+s*z, t*x*z-s*y, 0,
    t*x*y-s*z, t*y*y+c, t*y*z+s*x, 0,
    t*x*z+s*y, t*y*z-s*x, t*z*z+c, 0,
    0, 0, 0, 1,
  ]
}

function transformByMatrix(shapes: Shape[], value: Value, node: CallNode, ctx: EvalContext): Shape[] {
  const has2d = shapes.some(shape => shape.dimension === 2)
  const has3d = shapes.some(shape => shape.dimension === 3)
  let matrix2d: number[] | null = null
  if (has2d && isStableProfile(ctx)) {
    const resolved = resolveOpenScad2dMultmatrix(value)
    if (resolved === null) {
      warn(ctx, 'Invalid multmatrix() value leaves 2D children unchanged')
    } else {
      matrix2d = [...resolved]
    }
  }

  let matrix3d: number[] | null = null
  if (has3d || (has2d && !isStableProfile(ctx))) {
    if (!Array.isArray(value) || value.length < 3) evaluationError(ctx, node.p, 'multmatrix requires a 4x4 matrix')
    const rows = value.map(row => vectorValue(row, ctx, node.p, 'matrix row'))
    if (rows.some(row => row.length < 4)) evaluationError(ctx, node.p, 'multmatrix requires a 4x4 matrix')
    const matrix: number[] = []
    for (let column = 0; column < 4; column++) for (let row = 0; row < 4; row++) matrix.push(rows[row]?.[column] ?? (row === column ? 1 : 0))
    matrix3d = matrix
  }
  return shapes.map(shape => {
    if (shape.dimension === 2) {
      if (matrix2d === null) return shape
      return { ...shape, geometry: ctx.kernel.transform2(shape.geometry, matrix2d) }
    }
    return { ...shape, geometry: ctx.kernel.transform3(shape.geometry, matrix3d!) }
  })
}

function booleanShapes(
  sourceShapes: Shape[],
  operation: 'union' | 'intersection',
  ctx: EvalContext,
  p: number,
  diagnosticName: string = operation,
): Shape[] {
  let shapes = sourceShapes
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    if (!isStableProfile(ctx)) {
      evaluationError(ctx, p, `${diagnosticName}() cannot mix 2D and 3D children`)
    }
    warn(ctx, `${diagnosticName}() ignored child geometry with a different dimension`)
    if (operation === 'intersection') return []
    shapes = shapes.filter(shape => shape.dimension === dimension)
  }
  if (shapes.length === 1) return shapes
  ctx.control?.poll()
  if (dimension === 3) {
    const solids = shapes.map(shape => (shape as Shape3D).geometry)
    const geometry = kernelCall(ctx, p, () => ctx.kernel.boolean3(operation, solids))
    return [{ dimension: 3, geometry, color: shapes[0].color, entityId: currentEntityId(ctx) }]
  }
  const sections = shapes.map(shape => (shape as Shape2D).geometry)
  const geometry = kernelCall(ctx, p, () => ctx.kernel.boolean2(operation, sections))
  return [{ dimension: 2, geometry, color: shapes[0].color, entityId: currentEntityId(ctx) }]
}

async function differenceChildren(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  if (node.children.length === 0) return []
  const childContext = isStableProfile(ctx)
    ? enterStableStatementScope(node.children, ctx)
    : null
  const base = booleanShapes(
    childContext === null
      ? await evalNodes([node.children[0]], ctx)
      : await evalPreparedNodes([node.children[0]], childContext),
    'union',
    ctx,
    node.p,
  )
  const cutters = booleanShapes(
    childContext === null
      ? await evalNodes(node.children.slice(1), ctx)
      : await evalPreparedNodes(node.children.slice(1), childContext),
    'union',
    ctx,
    node.p,
  )
  if (base.length === 0 || cutters.length === 0) return base
  if (base[0].dimension !== cutters[0].dimension) {
    if (isStableProfile(ctx)) {
      warn(ctx, 'difference() ignored child geometry with a different dimension')
      return base
    }
    evaluationError(ctx, node.p, 'difference() cannot mix 2D and 3D children')
  }
  ctx.control?.poll()
  if (base[0].dimension === 3) {
    return [{
      dimension: 3,
      geometry: ctx.kernel.boolean3('difference', [base[0].geometry, (cutters[0] as Shape3D).geometry]),
      color: base[0].color,
      entityId: currentEntityId(ctx),
    }]
  }
  return [{
    dimension: 2,
    geometry: ctx.kernel.boolean2('difference', [base[0].geometry, (cutters[0] as Shape2D).geometry]),
    color: base[0].color,
    entityId: currentEntityId(ctx),
  }]
}

function hullShapes(shapes: Shape[], node: CallNode, ctx: EvalContext): Shape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'hull() cannot mix 2D and 3D children')
    warn(ctx, 'hull() ignored child geometry with a different dimension')
    shapes = shapes.filter(shape => shape.dimension === dimension)
  }
  ctx.control?.poll()
  if (dimension === 3) {
    const geometry = kernelCall(ctx, node.p, () => ctx.kernel.hull3(shapes.map(shape => (shape as Shape3D).geometry)))
    return [trackedSolid(geometry, shapes[0].color, node, ctx)]
  }
  return [{
    dimension: 2,
    geometry: kernelCall(ctx, node.p, () => ctx.kernel.hull2(shapes.map(shape => (shape as Shape2D).geometry))),
    color: shapes[0].color,
    entityId: currentEntityId(ctx),
  }]
}

async function resizeChildren(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  const evaluated = evaluateStableBuiltinModuleArguments(
    node,
    ['newsize', 'auto', 'convexity'],
    [],
    ctx,
  )
  const rawNewsize = evaluated.get('newsize')
  const rawAuto = evaluated.get('auto') ?? false
  const shapes = await evalNodes(node.children, ctx)
  if (shapes.length === 0) return shapes

  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    evaluationError(ctx, node.p, 'resize() cannot mix 2D and 3D children')
  }

  const axes = dimension === 3 ? 3 : 2
  const min = Array.from({ length: axes }, () => Infinity)
  const max = Array.from({ length: axes }, () => -Infinity)
  let hasGeometry = false
  for (const shape of shapes) {
    if (ctx.kernel.isEmpty(shape.geometry)) continue
    hasGeometry = true
    const bounds = ctx.kernel.bounds(shape.geometry)
    for (let axis = 0; axis < axes; axis++) {
      min[axis] = Math.min(min[axis], bounds.min[axis] ?? Infinity)
      max[axis] = Math.max(max[axis], bounds.max[axis] ?? -Infinity)
    }
  }
  if (!hasGeometry) return shapes

  const resolved = resolveOpenScadResize({
    dimension,
    extents: max.map((value, axis) => value - min[axis]),
    // Omitted newsize has the stable identity default. An explicitly invalid
    // non-vector still reaches the helper and produces a soft diagnostic.
    newsize: rawNewsize ?? [],
    auto: rawAuto,
  }, stableGeometrySemanticsContext(ctx))
  if (!resolved.valid || !resolved.applied) return shapes

  return shapes.map(shape => ({
    ...shape,
    geometry: ctx.kernel.scale(
      shape.geometry,
      shape.dimension === 3
        ? [resolved.scales[0], resolved.scales[1], resolved.scales[2]]
        : [resolved.scales[0], resolved.scales[1]],
    ),
  }))
}

function minkowskiShapes(shapes: Shape[], node: CallNode, ctx: EvalContext): Shape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    evaluationError(ctx, node.p, 'minkowski() cannot mix 2D and 3D children')
  }
  if (shapes.length === 1) return shapes

  try {
    if (dimension === 2) {
      const geometry = openScadMinkowski2dViaProduct(
        shapes.map(shape => (shape as Shape2D).geometry),
        {
          anchor: section => {
            // The kernel exposes Minkowski as an anchored dilation. Anchor at an
            // actual contour vertex (not a bounding-box corner, which may lie
            // outside a circle) so the normalized structuring set contains 0.
            const point = ctx.kernel.polygons(section)[0]?.[0]
            return point === undefined ? [0, 0] : [point[0], point[1]]
          },
          translate: (section, offset) => ctx.kernel.translate(section, offset),
          extrudeUnitPrism: section => ctx.kernel.linearExtrude(
            section, 1, 0, 0, [1, 1], false,
          ),
          minkowskiSum: (left, right) => ctx.kernel.minkowskiSum3(left, right),
          projectTo2d: solid => ctx.kernel.projection(solid, false),
        },
      )
      if (geometry === undefined) return []
      return [{
        dimension: 2,
        geometry,
        color: shapes[0].color,
        entityId: currentEntityId(ctx),
      }]
    }
    let geometry = (shapes[0] as Shape3D).geometry
    if (ctx.kernel.isEmpty(geometry)) return []
    for (let index = 1; index < shapes.length; index++) {
      const right = (shapes[index] as Shape3D).geometry
      if (ctx.kernel.isEmpty(right)) return []
      // The kernel exposes the right operand as an anchored dilation. Normalize
      // it around a point that is genuinely in the solid (a real mesh vertex),
      // then restore that translation after the ordered sum. Bounding-box
      // corners are insufficient because they need not belong to curved solids.
      const vertex = ctx.kernel.firstVertex3(right)
      if (vertex === null) return []
      const anchor: Vec3 = [vertex[0], vertex[1], vertex[2]]
      geometry = ctx.kernel.translate(
        ctx.kernel.minkowskiSum3(
          geometry,
          ctx.kernel.translate(right, [-anchor[0], -anchor[1], -anchor[2]]),
        ),
        anchor,
      )
    }
    return [trackedSolid(geometry, shapes[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `minkowski() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function compatibilityDxfProfile(
  node: CallNode,
  ctx: EvalContext,
  values: ReadonlyMap<string, Value>,
  fileValue: string,
  importScale: number,
): Shape[] {
  if (ctx.project === undefined || ctx.importAssets === undefined) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_PROJECT_REQUIRED',
      `${node.name}() file mode requires an OpenSCAD project inside the bounded project VFS.`,
      { specifier: fileValue, format: 'dxf' },
    )
  }
  if (!Number.isFinite(importScale) || importScale <= 0) return []

  const rawOrigin = values.get('origin')
  let origin: readonly [number, number] | undefined
  if (rawOrigin !== undefined) {
    if (Array.isArray(rawOrigin) && rawOrigin.length === 2
      && rawOrigin.every(value => typeof value === 'number' && Number.isFinite(value))) {
      origin = [rawOrigin[0] as number, rawOrigin[1] as number]
    } else {
      warn(ctx, `${node.name}(..., origin=${formatOpenScadValue(rawOrigin)}) could not be converted`)
    }
  }
  const layer = compatibilityString(values.get('layer'))
  const importer = node.sourcePath ?? ctx.project.entrypoint
  let assetPath: string | undefined
  try {
    const loaded = loadPreparedOpenScadImport(
      ctx.project,
      ctx.importAssets,
      importer,
      fileValue,
      {
        forcedFormat: 'dxf',
        layer,
        ...(origin === undefined ? {} : { origin }),
        scale: importScale,
      },
    )
    assetPath = resolveOpenScadProjectPath(importer, fileValue)
    if (loaded.dimension !== 2) {
      return importCallError(
        node,
        ctx,
        'E_IMPORT_INVALID_DATA',
        `${node.name}() file mode requires 2D DXF geometry.`,
        { specifier: fileValue, assetPath, format: 'dxf' },
      )
    }
    const sections = loaded.regions.map(region => ctx.kernel.polygon(
      region.contours.map(contour => contour.map(point => [point[0], point[1]] as Vec2)),
      region.fillRule === 'evenodd' ? 'EvenOdd' : 'NonZero',
    ))
    if (sections.length === 0) return []
    const geometry = sections.length === 1
      ? sections[0]
      : ctx.kernel.boolean2('union', sections)
    if (ctx.kernel.isEmpty(geometry)) return []
    return [{
      dimension: 2,
      geometry,
      color: nextColor(),
      entityId: currentEntityId(ctx),
    }]
  } catch (error) {
    if (error instanceof OpenScadImportPositionedError) throw error
    if (error instanceof OpenScadImportError) {
      return importCallError(node, ctx, error.code, error.message, {
        specifier: error.specifier,
        ...(error.assetPath === undefined ? {} : { assetPath: error.assetPath }),
        ...(error.format === undefined ? {} : { format: error.format }),
        ...(error.limit === undefined ? {} : { limit: error.limit }),
        ...(error.actual === undefined ? {} : { actual: error.actual }),
      })
    }
    return importCallError(
      node,
      ctx,
      'E_IMPORT_INVALID_DATA',
      `${node.name}() could not construct its DXF profile: ${error instanceof Error ? error.message : String(error)}`,
      { specifier: fileValue, ...(assetPath === undefined ? {} : { assetPath }), format: 'dxf' },
    )
  }
}

function stableLinearExtrudeSections(
  node: CallNode,
  ctx: EvalContext,
  evaluated: ReadonlyMap<string, Value>,
  sections: Shape[],
): Shape[] {
  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, `${node.name}() requires 2D children`)
  const profilePoints = ctx.kernel.polygons(sections[0].geometry).flatMap(polygon => (
    polygon.map(point => [point[0], point[1]] as const)
  ))
  const fragmentInput = stableFragmentInputFromEvaluated(evaluated, ctx, 0)
  const resolved = resolveOpenScadLinearExtrude({
    height: evaluated.get('height'),
    scale: evaluated.get('scale'),
    center: evaluated.get('center'),
    twist: evaluated.get('twist'),
    slices: evaluated.get('slices'),
    fn: fragmentInput.fn,
    fa: fragmentInput.fa,
    fs: fragmentInput.fs,
    profilePoints,
    maximumSlices: MAX_EXTRUDE_SLICES,
  }, stableGeometrySemanticsContext(ctx))
  if (resolved.reduced) ctx.reduced.value = true
  if (resolved.empty) return []
  try {
    const geometry = ctx.kernel.linearExtrude(
      sections[0].geometry,
      resolved.height,
      resolved.manifoldNDivisions,
      resolved.twist,
      [...resolved.scale] as Vec2,
      resolved.center,
    )
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `${node.name}() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

async function linearExtrude(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  const evaluated = isStableProfile(ctx)
    ? evaluateStableBuiltinModuleArguments(
        node,
        ['height', 'center', 'convexity', 'twist', 'slices', 'scale'],
        ['$fn', '$fa', '$fs'],
        ctx,
      )
    : null
  const sections = booleanShapes(await evalNodes(node.children, ctx), 'union', ctx, node.p)

  if (evaluated !== null) {
    return stableLinearExtrudeSections(node, ctx, evaluated, sections)
  }

  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, 'linear_extrude() requires 2D children')

  const height = finiteNumber(arg(node, 'height', 0, 1, ctx), ctx, node.p, 'extrusion height')
  if (height <= 0) evaluationError(ctx, node.p, 'linear_extrude() height must be positive')
  const twist = finiteNumber(arg(node, 'twist', -1, 0, ctx), ctx, node.p, 'extrusion twist')
  // Cap slices: the value goes straight into the geometry kernel, which
  // allocates per-slice cross-sections long before MAX_TRIANGLES can fire.
  const slices = Math.min(MAX_EXTRUDE_SLICES, Math.max(0, Math.trunc(finiteNumber(arg(node, 'slices', -1, 0, ctx), ctx, node.p, 'extrusion slices'))))
  const rawScale = arg(node, 'scale', -1, [1, 1], ctx)
  const scaleValues = Array.isArray(rawScale) ? vectorValue(rawScale, ctx, node.p, 'extrusion scale') : [finiteNumber(rawScale, ctx, node.p, 'extrusion scale')]
  const scale: Vec2 = [scaleValues[0] ?? 1, scaleValues[1] ?? scaleValues[0] ?? 1]
  try {
    const geometry = ctx.kernel.linearExtrude(sections[0].geometry, height, slices, twist, scale, arg(node, 'center', -1, false, ctx) === true)
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `linear_extrude() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

async function dxfLinearExtrude(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  const evaluated = evaluateStableCompatibilityArguments(
    node,
    ['file', 'layer', 'height', 'origin', 'scale', 'center', 'twist', 'slices'],
    ['convexity', '$fn', '$fa', '$fs'],
    ctx,
  )
  const values = new Map(evaluated.values)
  if (values.get('height') === undefined) {
    const first = evaluated.authored[0]
    const firstValue = first?.name === undefined ? evaluated.evaluated.get(first) : undefined
    if (typeof firstValue === 'number' && Number.isFinite(firstValue)) values.set('height', firstValue)
  }

  const rawScale = values.get('scale')
  let importScale = 1
  if (typeof rawScale === 'number' && Number.isFinite(rawScale)) {
    importScale = Math.max(0, rawScale)
  } else if (Array.isArray(rawScale) && rawScale.length === 2) {
    const scaleX = rawScale[0]
    const scaleY = rawScale[1]
    if (typeof scaleX === 'number' && Number.isFinite(scaleX)
      && typeof scaleY === 'number' && Number.isFinite(scaleY)) {
      importScale = Math.max(0, scaleX)
    }
  }
  const file = values.get('file')
  const sourceSections = typeof file === 'string' && file.length > 0
    ? compatibilityDxfProfile(node, ctx, values, file, importScale)
    : await evalNodes(node.children, ctx)
  const sections = booleanShapes(sourceSections, 'union', ctx, node.p, node.name)
  return stableLinearExtrudeSections(node, ctx, values, sections)
}

function stableRotateExtrudeSections(
  node: CallNode,
  ctx: EvalContext,
  evaluated: ReadonlyMap<string, Value>,
  sections: Shape[],
): Shape[] {
  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, `${node.name}() requires 2D children`)
  const bounds = ctx.kernel.bounds(sections[0].geometry)
  const radius = Math.max(Math.abs(bounds.min[0] ?? 0), Math.abs(bounds.max[0] ?? 0))
  const fragmentInput = stableFragmentInputFromEvaluated(evaluated, ctx, radius)
  const resolved = resolveOpenScadRotateExtrude({
    angle: evaluated.get('angle'),
    profileXMin: bounds.min[0],
    profileXMax: bounds.max[0],
    fn: fragmentInput.fn,
    fa: fragmentInput.fa,
    fs: fragmentInput.fs,
    quality: ctx.quality,
  }, stableGeometrySemanticsContext(ctx))
  if (resolved.crossesAxis) {
    evaluationError(ctx, node.p, `${node.name}() profile may not cross the Y axis`)
  }
  if (resolved.reduced) ctx.reduced.value = true
  if (resolved.empty) return []
  const profile = resolved.reflectProfileX
    ? ctx.kernel.transform2(sections[0].geometry, [
        -1, 0, 0,
        0, 1, 0,
        0, 0, 1,
      ])
    : sections[0].geometry
  try {
    let geometry = ctx.kernel.rotateExtrude(
      profile,
      resolved.circularSegments,
      resolved.angle,
    )
    if (resolved.postRotateDegrees !== 0) {
      geometry = ctx.kernel.rotate(geometry, [0, 0, resolved.postRotateDegrees])
    }
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `${node.name}() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

async function dxfRotateExtrude(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  const evaluated = evaluateStableCompatibilityArguments(
    node,
    ['file', 'layer', 'origin', 'scale'],
    ['convexity', 'angle', '$fn', '$fa', '$fs'],
    ctx,
  )
  const values = new Map(evaluated.values)
  const rawScale = values.get('scale')
  const importScale = typeof rawScale === 'number' && Number.isFinite(rawScale) && rawScale > 0
    ? rawScale
    : 1
  const rawFile = values.get('file')
  const file = rawFile === undefined ? '' : compatibilityString(rawFile)
  const sourceSections = file.length > 0
    ? compatibilityDxfProfile(node, ctx, values, file, importScale)
    : await evalNodes(node.children, ctx)
  const sections = booleanShapes(sourceSections, 'union', ctx, node.p, node.name)
  return stableRotateExtrudeSections(node, ctx, values, sections)
}

async function rotateExtrude(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  const evaluated = isStableProfile(ctx)
    ? evaluateStableBuiltinModuleArguments(
        node,
        ['angle', 'convexity'],
        ['$fn', '$fa', '$fs'],
        ctx,
      )
    : null
  const sections = booleanShapes(await evalNodes(node.children, ctx), 'union', ctx, node.p)
  if (evaluated !== null) {
    return stableRotateExtrudeSections(node, ctx, evaluated, sections)
  }

  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, 'rotate_extrude() requires 2D children')
  const bounds = ctx.kernel.bounds(sections[0].geometry)
  const radius = Math.max(Math.abs(bounds.min[0] ?? 0), Math.abs(bounds.max[0] ?? 0))
  const angle = finiteNumber(arg(node, 'angle', -1, 360, ctx), ctx, node.p, 'revolve angle')
  const circularSegments = segments(node, ctx, 48, 3, radius)
  try {
    const geometry = ctx.kernel.rotateExtrude(sections[0].geometry, circularSegments, angle)
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `rotate_extrude() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

async function evalFor(node: CallNode, ctx: EvalContext, diagnosticName: 'for' | 'intersection_for'): Promise<Shape[]> {
  const control = ctx.control
  if (isStableProfile(ctx)) {
    const bindings = callExpressionArguments(node)
    const output: Shape[] = []
    const occurrences = new Map<string, number>()
    let iterationsSinceYield = 0
    const visit = async (
      bindingIndex: number,
      iterationContext: EvalContext,
      path: readonly string[],
    ): Promise<void> => {
      if (bindingIndex >= bindings.length) {
        const valueKey = path.join(',')
        const occurrence = occurrences.get(valueKey) ?? 0
        occurrences.set(valueKey, occurrence + 1)
        if (control && iterationsSinceYield >= YIELD_EVERY_STATEMENTS) {
          iterationsSinceYield = 0
          await control.yieldIfDue()
        }
        iterationsSinceYield++
        control?.poll()
        output.push(...await evalNodes(node.children, {
          ...iterationContext,
          instancePath: `${ctx.instancePath}>loop:${valueKey}#${occurrence}`,
        }, false))
        if (output.length > MAX_SHAPES) {
          evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
        }
        return
      }
      const binding = bindings[bindingIndex]
      const values = stableIterable(
        evalExpression(binding.value, iterationContext),
        iterationContext,
        binding.p,
      )
      if (binding.name === undefined) {
        warn(ctx, `Ignoring ${diagnosticName}() iterator without variable name`)
        return
      }
      for (const value of values) {
        if (++ctx.budget.ops > MAX_EVAL_OPS) {
          evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
        }
        const env = new Map(iterationContext.env)
        env.set(binding.name, value)
        const segment = `${encodeURIComponent(binding.name)}=${encodeURIComponent(identityValue(value))}`
        await visit(bindingIndex + 1, stableOverlayContext(iterationContext, env), [...path, segment])
      }
    }
    await visit(0, ctx, [])
    return output
  }
  const entries = Object.entries(node.args).filter(([name]) => !name.startsWith('_'))
  if (entries.length !== 1) evaluationError(ctx, node.p, `${diagnosticName}() currently requires one named iterator`)
  const [name, expression] = entries[0]
  const values = evalExpression(expression, ctx)
  if (!Array.isArray(values)) evaluationError(ctx, node.p, `${diagnosticName}() iterator must be a vector or range`)
  const output: Shape[] = []
  const occurrences = new Map<string, number>()
  let iterationsSinceYield = 0
  for (const value of values) {
    if (control && iterationsSinceYield >= YIELD_EVERY_STATEMENTS) {
      iterationsSinceYield = 0
      await control.yieldIfDue()
    }
    iterationsSinceYield++
    control?.poll()
    if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
    const env = new Map(ctx.env)
    env.set(name, value)
    const valueKey = encodeURIComponent(identityValue(value))
    const occurrence = occurrences.get(valueKey) ?? 0
    occurrences.set(valueKey, occurrence + 1)
    output.push(...await evalNodes(node.children, {
      ...ctx,
      env,
      instancePath: `${ctx.instancePath}>loop:${encodeURIComponent(name)}=${valueKey}#${occurrence}`,
    }, false))
    if (output.length > MAX_SHAPES) evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
  }
  return output
}

function identityValue(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(identityValue).join(',')}]`
  if (isFunctionValue(value)) return `function:${value.name ?? '<anonymous>'}`
  if (isOpenScadRange(value)) return `range:${formatOpenScadValue(value)}`
  if (value === undefined) return 'undef'
  if (typeof value === 'string') return JSON.stringify(value)
  return String(value)
}

async function evalUserModule(
  call: CallNode,
  module: ModuleNode,
  ctx: EvalContext,
  definitionScope?: OpenScadStableScope,
): Promise<Shape[]> {
  let env = new Map(ctx.env)
  if (isStableProfile(ctx)) {
    if (definitionScope === undefined) {
      evaluationError(ctx, call.p, `Stable module ${module.name}() is missing its lexical scope`)
    }
    const moduleStack = [...ctx.moduleStack, module.name]
    const definitionEnv = new Map(definitionScope.env)
    overlayDynamicVariables(definitionEnv, ctx.env)
    definitionEnv.set('$children', call.children.length)
    definitionEnv.set('$parent_modules', moduleStack.length)
    const definitionContext: EvalContext = {
      ...ctx,
      env: definitionEnv,
      stableScope: definitionScope,
      scopeVisibleBefore: Number.POSITIVE_INFINITY,
      moduleStack,
    }
    const args = callExpressionArguments(call)
    const resolved = resolveStableExpressionArguments(
      args,
      module.params.map(parameter => parameter.name),
      ctx,
    )
    const callerValues = new Map<ExpressionArgument, Value>()
    for (const argument of args) callerValues.set(argument, evalExpression(argument.value, ctx))
    const parameterValues = new Map<string, Value>()
    for (const parameter of module.params) {
      const supplied = resolved.get(parameter.name)
      const value = supplied !== undefined
        ? callerValues.get(supplied)
        : parameter.defaultValue !== undefined
          ? evalExpression(parameter.defaultValue, { ...definitionContext, env: new Map(definitionEnv) })
          : undefined
      parameterValues.set(parameter.name, value)
    }
    env = new Map(definitionEnv)
    for (const [name, value] of parameterValues) env.set(name, value)
    return booleanShapes(await evalNodes(module.children, {
      ...definitionContext,
      env,
      callChildren: {
        statements: call.children,
        env: new Map(ctx.env),
        scope: ctx.stableScope ?? definitionScope,
        continuation: ctx.callChildren,
      },
    }, false), 'union', ctx, call.p, module.name)
  } else {
    const positional = Object.entries(call.args).filter(([name]) => name.startsWith('_')).sort(([a], [b]) => Number(a.slice(1)) - Number(b.slice(1)))
    for (let i = 0; i < module.params.length; i++) {
      const param = module.params[i]
      const expression = call.args[param.name] ?? positional[i]?.[1] ?? param.defaultValue
      env.set(param.name, expression ? evalExpression(expression, { ...ctx, env }) : undefined)
    }
  }
  return await evalNodes(module.children, {
    ...ctx,
    env,
    callChildren: call.children,
    moduleStack: [...ctx.moduleStack, module.name],
  }, false)
}

function parseLegacyColor(value: Value, ctx: EvalContext, p: number): RGBA {
  if (Array.isArray(value)) {
    const channels = vectorValue(value, ctx, p, 'color')
    return [clamp01(channels[0] ?? 0.5), clamp01(channels[1] ?? 0.5), clamp01(channels[2] ?? 0.5), clamp01(channels[3] ?? 1)]
  }
  if (typeof value !== 'string') evaluationError(ctx, p, 'color() expects a name or RGB(A) vector')
  const named = CSS_COLORS[value.toLowerCase()]
  if (named) return [...named]
  const match = value.match(/^#([0-9a-f]{6}|[0-9a-f]{8})$/i)
  if (match) {
    const hex = match[1]
    return [parseInt(hex.slice(0, 2), 16) / 255, parseInt(hex.slice(2, 4), 16) / 255, parseInt(hex.slice(4, 6), 16) / 255, hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1]
  }
  evaluationError(ctx, p, `Unknown color ${value}`)
}

/**
 * Warm the official geometry kernel, cached per JS realm. Exported so a
 * hosting worker can eagerly compile it at startup instead of paying the
 * download+compile cost on the first request. A rejected load is NOT cached:
 * one transient network failure must not brick every future parse, so the
 * cached promise is cleared on rejection and the next call retries.
 */
export function warmGeometryKernel(): Promise<void> {
  return defaultGeometryKernel.warm().then(() => undefined)
}

async function parseInternal(
  source: string,
  options: ParseOptions,
  compileAst: () => readonly Statement[] = () => compileOpenSCAD(source, {
    languageProfile: options.languageProfile,
  }),
  project?: OpenScadProject,
): Promise<ParseResult> {
  const now = options.now ?? (() => performance.now())
  const startedAt = now()
  if (project === undefined && source.length > MAX_SOURCE_LENGTH) {
    throw new OpenSCADParseError(source, 0, `Source exceeds ${MAX_SOURCE_LENGTH.toLocaleString()} characters`)
  }
  resetPalette()
  const languageProfile = options.languageProfile ?? 'openscad-viewer-subset@1'
  let ast: readonly Statement[]
  let bound
  let parseMs: number
  let bindMs: number
  if (project === undefined) {
    const prepared = prepareOpenScadFrontEnd(source, {
      languageProfile,
      compile: () => compileAst(),
      now,
    })
    ast = prepared.program
    bound = prepared.bound
    parseMs = prepared.compileMs
    bindMs = prepared.bindMs
  } else {
    ast = compileAst()
    const compiledAt = now()
    bound = bindOpenScad(ast, { languageProfile, source })
    const boundAt = now()
    parseMs = Math.max(0, compiledAt - startedAt)
    bindMs = Math.max(0, boundAt - compiledAt)
  }
  const forcedImportAssets = project === undefined
    ? []
    : stableCompatibilityForcedAssets(project, ast)
  const [importAssets, surfaceAssets, textAssets] = project === undefined
    ? [undefined, undefined, undefined]
    : await Promise.all([
        prepareOpenScadImportAssets(project, { forcedAssets: forcedImportAssets }),
        prepareOpenScadSurfaceAssets(project),
        prepareOpenScadTextAssets(project),
      ])
  const parsedAt = now()
  const kernelSession = await defaultGeometryKernel.openEvalSession()
  // Recording costs only a small JSON graph, but it is opt-in because the result
  // crosses a protocol that validates an exact set of keys.
  const exactRecorder = options.recordExactSolids === true
    ? createBrepRecordingKernelOps(kernelSession.kernel)
    : null
  const warnings: string[] = []
  const modules = new Map(bound.modules)
  const functions = new Map(bound.functions)
  const quality = options.quality ?? 'full'
  const env = languageProfile === 'openscad/stable-2021.01'
    ? new Map<string, Value>(Array.from(
        createOpenScadStableRuntimeVariables({ quality, animationTime: options.animationTime }),
        ([name, value]) => [name, Array.isArray(value) ? [...value] : value as Value],
      ))
    : new Map<string, Value>([['$fn', 0], ['$fa', 12], ['$fs', 2]])
  const sourceReferences = new Map<number, MeshSourceReference>()
  const reduced = { value: false }
  let ctx: EvalContext = {
    kernel: exactRecorder?.ops ?? kernelSession.kernel,
    source,
    project,
    importAssets,
    surfaceAssets,
    textAssets,
    languageProfile,
    env,
    functions,
    modules,
    warnings,
    quality,
    sourceReferences,
    depth: 0,
    functionStack: [],
    moduleStack: [],
    budget: { ops: 0 },
    reduced,
    instancePath: 'root',
    valueBudget: { used: 0 },
    valueWeights: new WeakMap(),
    valueDepths: new WeakMap(),
  }
  if (isStableProfile(ctx)) ctx = enterStableStatementScope(ast, ctx)
  const initializedAt = now()
  const control = new CooperativeCheckpoint(options.shouldAbort, options.onYield, now, options.yieldControl)
  ctx = { ...ctx, control }

  try {
    let shapes = await evalTopLevel(ast, ctx, control)
    if (isStableProfile(ctx)) shapes = booleanShapes(shapes, 'union', ctx, 0, 'union')
    // A single giant statement may occupy a whole macrotask. Always yield
    // before post-processing so a queued cancel can skip mesh extraction.
    await control.yieldIfDue(true)
    const evaluatedAt = now()
    // Mesh extraction (normals/BVH/edges) is often the dominant cost — a
    // superseded request must not pay it in full before the newest starts.
    if (options.shouldAbort?.()) throw new AbortedError()
    const sections = shapes.filter(shape => shape.dimension === 2)
    if (sections.length) warn(ctx, `${sections.length} top-level 2D object(s) are not displayed; wrap them in linear_extrude() or rotate_extrude()`)
    const solids = shapes.filter((shape): shape is Shape3D => shape.dimension === 3 && !ctx.kernel.isEmpty(shape.geometry))
    const meshes: MeshData[] = []
    let volume = 0
    let surfaceArea = 0
    let triangleCount = 0
    const exactRoots: NonNullable<ParseResult['exactSolids']>['roots'][number][] = []
    for (const [index, shape] of solids.entries()) {
      if (options.shouldAbort?.()) throw new AbortedError()
      // Resolved here, while the handle is still alive and before its mesh is extracted.
      if (exactRecorder) {
        const name = `Body ${index + 1}`
        const resolved = exactRecorder.recording.resolve(shape.geometry)
        exactRoots.push('id' in resolved ? { name, id: resolved.id } : { name, inexact: resolved.inexact })
      }
      const analysis = ctx.kernel.analyzeSolid(shape.geometry)
      volume += analysis.volume
      surfaceArea += analysis.surfaceArea
      const mesh = analysis.mesh
      await control.yieldIfDue()
      triangleCount += mesh.numTri
      if (triangleCount > MAX_TRIANGLES) evaluationError(ctx, 0, `Rendered model exceeds ${MAX_TRIANGLES.toLocaleString()} triangles`)
      if (mesh.numProp < 6) evaluationError(ctx, 0, 'Geometry kernel did not produce normals')
      const vertices = new Float32Array(mesh.numVert * 6)
      if (mesh.numProp === 6) {
        // Exactly position+normal: one memcpy instead of 6 writes per vertex.
        vertices.set(mesh.vertProperties.subarray(0, mesh.numVert * 6))
      } else {
        for (let vertex = 0; vertex < mesh.numVert; vertex++) {
          const sourceOffset = vertex * mesh.numProp
          const targetOffset = vertex * 6
          for (let channel = 0; channel < 6; channel++) vertices[targetOffset + channel] = mesh.vertProperties[sourceOffset + channel]
          if ((vertex & 0x3fff) === 0x3fff) await control.yieldIfDue()
        }
      }
      const indices = new Uint32Array(mesh.triVerts)
      const bvh = buildMeshBvh(vertices, indices)
      await control.yieldIfDue()
      const semanticEdges = extractSemanticEdges(vertices, indices, {
        creaseAngleDegrees: 30,
        mergeFromVert: mesh.mergeFromVert,
        mergeToVert: mesh.mergeToVert,
      })
      await control.yieldIfDue()
      const provenance: MeshProvenanceRun[] = []
      for (let run = 0; run < mesh.runOriginalID.length; run++) {
        const triangleStart = (mesh.runIndex[run] ?? 0) / 3
        const triangleEnd = (mesh.runIndex[run + 1] ?? mesh.triVerts.length) / 3
        if (triangleEnd <= triangleStart) continue
        const originalId = mesh.runOriginalID[run]
        provenance.push({
          triangleStart,
          triangleEnd,
          source: sourceReferences.get(originalId) ?? null,
          backside: ((mesh.runFlags[run] ?? 0) & 1) !== 0,
        })
        if ((run & 0x1fff) === 0x1fff) await control.yieldIfDue()
      }
      if (provenance.length === 0 && mesh.numTri > 0) {
        provenance.push({ triangleStart: 0, triangleEnd: mesh.numTri, source: null, backside: false })
      }
      meshes.push({
        entityId: shape.entityId,
        geometryAssetId: geometryAssetId(vertices, indices),
        vertices,
        indices,
        bvh,
        edgeIndices: semanticEdges.indices,
        color: [...shape.color],
        transform: identity(),
        faceIds: new Uint32Array(mesh.faceID),
        provenance,
        topology: semanticEdges.diagnostics,
      })
      await control.yieldIfDue()
    }
    const analyzedAt = now()
    return {
      meshes, warnings, volume, surfaceArea, quality, reduced: reduced.value,
      ...(exactRecorder ? { exactSolids: { nodes: exactRecorder.recording.nodes, roots: exactRoots } } : {}),
      timings: {
        parseMs,
        bindMs,
        initializeMs: Math.max(0, initializedAt - parsedAt),
        evaluateMs: Math.max(0, evaluatedAt - initializedAt),
        analyzeMs: Math.max(0, analyzedAt - evaluatedAt),
      },
    }
  } finally {
    kernelSession.dispose()
  }
}

let parseQueue: Promise<void> = Promise.resolve()

/** Parse and evaluate the supported OpenSCAD subset in a serialized WASM scope. */
export function parseOpenSCAD(source: string, options: ParseOptions = {}): Promise<ParseResult> {
  const run = async () => {
    if (/^\s*\/\/\s*@modelgraph-text\/1\b/.test(source)) {
      const { compileModelGraphText } = await import('./modelGraphText')
      const compiled = compileModelGraphText(source)
      if (compiled.execution_target === 'own-nurbs') {
        const { buildTextNurbsScene } = await import('./modelGraphTextScene')
        if (options.shouldAbort?.()) throw new AbortedError()
        const result = await buildTextNurbsScene(compiled.document, options.quality)
        if (options.shouldAbort?.()) throw new AbortedError()
        return result
      }
      const result = await parseInternal(compiled.source, options)
      requireModelGraphChecks(await evaluateModelGraphGeometry(compiled.geometry_assertions, result.meshes, async source => (await parseInternal(source, options)).meshes))
      // Generated SCAD spans are not spans in the authored compact document.
      for (const mesh of result.meshes) for (const run of mesh.provenance) run.source = null
      return result
    }
    return parseInternal(source, options)
  }
  const result = parseQueue.then(run, run)
  parseQueue = result.then(() => undefined, () => undefined)
  return result
}

/** Compile and evaluate one bounded project snapshot with the independent full profile. */
export function parseOpenScadProject(
  project: OpenScadProject,
  options: ParseOpenScadProjectOptions = {},
): Promise<ParseResult> {
  const run = () => parseInternal(
    project.readEntrypoint().source,
    { ...options, languageProfile: 'openscad/stable-2021.01' },
    () => compileOpenScadProject(project),
    project,
  )
  const result = parseQueue.then(run, run)
  parseQueue = result.then(() => undefined, () => undefined)
  return result
}
