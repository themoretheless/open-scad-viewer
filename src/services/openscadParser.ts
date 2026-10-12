import { isModelGraphText } from './modelGraphTextDetect'
import { boundedSceneEntityId } from '../core/boundedSceneEntityId'
import { brepGearFaceCount } from './geometry/brep'
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
import { clamp01, identity, type Mat4 } from './math3d'
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
  type ModuleNode,
  type OpenScadLanguageProfile,
  type Statement,
  type Value,
} from './openscadCompiler'
import {
  formatOpenScadValue,
  isOpenScadRange,
  openScadIndex,
  openScadMember,
  openScadTruthy,
  openScadUnary,
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
  MAX_EVAL_DEPTH,
  MAX_EVAL_OPS,
  MAX_EVALUATED_VALUE_UNITS,
  MAX_EXPRESSION_DEPTH,
  MAX_RANGE_ITEMS,
  MAX_VALUE_ELEMENTS,
  appendComprehensionValues,
  compactDiagnosticText,
  createStableExpressionEvaluators,
  evaluationError,
  isFunctionValue,
  isListComprehensionExpression,
  isStableProfile,
  overlayDynamicVariables,
  registerArrayValue,
  stableOverlayContext,
  type StableFunctionValue,
} from './openScadStableExpressionEval'
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
import { CSS_COLORS, nextColor, resetPalette, type RGBA } from './openscadColors'
import { evalExpression, resolveStableVariable, warn } from './openscadParser/isExclusiveView'
import { evalBuiltin, finiteNumber } from './openscadParser/evalBuiltin'
import { parseInternal } from './openscadParser/warmGeometryKernel'
export { warmGeometryKernel } from './openscadParser/warmGeometryKernel'

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
  onSweepPreview?: import('./modelGraphNurbsKernel').OwnNurbsBuildControl['onSweepPreview']
  /** Injectable monotonic clock for deterministic phase-timing tests. */
  now?: () => number
  /** Injectable macrotask yield used by cooperative-cancellation tests. */
  yieldControl?: () => Promise<void>
}

export type ParseOpenScadProjectOptions = Omit<ParseOptions, 'languageProfile'>

/** @deprecated Prefer the kernel-neutral GeometryEvaluationResult contract. */
export type ParseResult = GeometryEvaluationResult

export const MAX_SOURCE_LENGTH = 250_000
const MAX_AST_NODES = 25_000
export const MAX_SHAPES = 1_000
export const MAX_TRIANGLES = 750_000
export const MAX_FN = 256
/** Cap on linear_extrude slices — passed straight into the geometry kernel,
 * which allocates per-slice cross-sections before MAX_TRIANGLES can fire. */
export const MAX_EXTRUDE_SLICES = 512

export type Vec2 = [number, number]
export type Vec3 = [number, number, number]

export interface EvalContext {
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

export interface Shape2D { dimension: 2; geometry: CadKernelHandle; color: RGBA; entityId: SceneEntityId }
export interface Shape3D { dimension: 3; geometry: CadKernelHandle; color: RGBA; entityId: SceneEntityId }
export type Shape = Shape2D | Shape3D

export class StableViewportRootSelection {
  constructor(readonly shapes: Shape[]) {}
}

export class StableBuiltinValueError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'StableBuiltinValueError'
  }
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

export interface EvaluatedStableCompatibilityArguments {
  readonly authored: readonly ExpressionArgument[]
  readonly values: ReadonlyMap<string, Value>
  readonly evaluated: ReadonlyMap<ExpressionArgument, Value>
}

export interface BoundAssertArguments {
  condition: Expr
  conditionText: string
  message?: Expr
}

export const STABLE_COMPATIBILITY_MODULE_FORMAT: Readonly<Partial<Record<string, OpenScad2021LegacyImportFormat>>> = Object.freeze({
  import_stl: 'stl',
  import_off: 'off',
  import_dxf: 'dxf',
  dxf_linear_extrude: 'dxf',
  dxf_rotate_extrude: 'dxf',
})

/** Yield to the event loop after this many statements or loop iterations… */
export const YIELD_EVERY_STATEMENTS = 25
/** …or once this much wall-clock time has elapsed since the last yield. */
const YIELD_EVERY_MS = 50

export class CooperativeCheckpoint {
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

export const TEXT_POSITIONAL_PARAMETERS = Object.freeze([
  'text',
  'size',
  'font',
] as const)

export const TEXT_NAMED_PARAMETERS = new Set([
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

export const IMPORT_POSITIONAL_PARAMETERS = Object.freeze([
  'file',
  'layer',
  'convexity',
  'origin',
  'scale',
] as const)

export const IMPORT_NAMED_PARAMETERS = new Set([
  ...IMPORT_POSITIONAL_PARAMETERS,
  'width',
  'height',
  'center',
  'dpi',
  '$fn',
  '$fa',
  '$fs',
])

let parseQueue: Promise<void> = Promise.resolve()

/** Parse and evaluate the supported OpenSCAD subset in a serialized WASM scope. */
export function parseOpenSCAD(source: string, options: ParseOptions = {}): Promise<ParseResult> {
  const run = async () => {
    if (isModelGraphText(source)) {
      const { compileModelGraphText } = await import('./modelGraphText')
      const compiled = compileModelGraphText(source)
      if (compiled.execution_target === 'own-nurbs') {
        const { buildTextNurbsScene } = await import('./modelGraphTextScene')
        if (options.shouldAbort?.()) throw new AbortedError()
        const result = await buildTextNurbsScene(compiled.document, options.quality, {shouldAbort:options.shouldAbort,onYield:options.onYield,onSweepPreview:options.onSweepPreview})
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
