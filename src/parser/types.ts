import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import type { MeshSourceReference, SceneEntityId } from '../core/mesh'
import type { CadKernelHandle, CadKernelOps } from '../services/cadKernelOps'
import type {
  CallNode,
  FunctionNode,
  ModuleNode,
  OpenScadLanguageProfile,
  Statement,
  Value,
} from '../services/openscadCompiler'
import { AbortedError } from '../services/openscadErrors'
import type { RGBA } from '../services/openscadColors'
import type { PreparedOpenScadImportAssets } from '../services/openScadImport'
import type { OpenScadProject } from '../services/openScadProject'
import type { OpenScadStableScope } from '../services/openScadStableScope'
import type { PreparedOpenScadSurfaceAssets } from '../services/openScadSurface'
import type { PreparedOpenScadTextAssets } from '../services/openScadText'

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
  /** Approximate sweep levels; never authoritative scene results. */
  onSweepPreview?: import('../services/rushGraphNurbsKernel').OwnNurbsBuildControl['onSweepPreview']
  /** Injectable monotonic clock for deterministic phase-timing tests. */
  now?: () => number
  /** Injectable macrotask yield used by cooperative-cancellation tests. */
  yieldControl?: () => Promise<void>
}

export type ParseOpenScadProjectOptions = Omit<ParseOptions, 'languageProfile'>

/** @deprecated Prefer the kernel-neutral GeometryEvaluationResult contract. */
export type ParseResult = GeometryEvaluationResult

export const MAX_SOURCE_LENGTH = 250_000
export const MAX_AST_NODES = 25_000
export const MAX_SHAPES = 1_000
export const MAX_TRIANGLES = 750_000
/** Cap on linear_extrude slices — passed straight into the geometry kernel,
 * which allocates per-slice cross-sections before MAX_TRIANGLES can fire. */
export const MAX_EXTRUDE_SLICES = 512

/** Yield to the event loop after this many statements or loop iterations… */
export const YIELD_EVERY_STATEMENTS = 25
/** …or once this much wall-clock time has elapsed since the last yield. */
export const YIELD_EVERY_MS = 50

/** True when `view` is a fresh array of exactly `length` elements that owns its
 * whole buffer, so it can be published or transferred without a copy. */
export function isExclusiveView(view: Float32Array | Uint32Array, length: number): boolean {
  return view.length === length && view.byteOffset === 0 && view.buffer.byteLength === view.byteLength
}

export type Vec2 = [number, number]
export type Vec3 = [number, number, number]

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

export interface PassedCallChildren {
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
