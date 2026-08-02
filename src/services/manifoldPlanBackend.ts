import type { SemanticNode, SemanticValueType } from '../core/semanticProgram'
import {
  loadManifoldKernelOps,
  type ManifoldKernelHandle,
  type ManifoldKernelOps,
  type ManifoldKernelSolidAnalysis,
} from './manifoldKernelOps'
import type {
  SemanticBackendBeginContext,
  SemanticBackendCloseOutcome,
  SemanticBackendCloseResult,
  SemanticBackendContext,
  SemanticBackendEvaluation,
  SemanticBackendPayloadLease,
  SemanticBackendResultLease,
  SemanticBackendSession,
  SemanticCarrierKey,
  SemanticProgramBackend,
  SemanticRuntimeValue,
} from './semanticProgramExecutor'

const PRESERVING_EVIDENCE = Object.freeze({ tag: 'representation-preserving' as const })

export const MANIFOLD_PLAN_CARRIERS = Object.freeze([
  'solid-set/d3/mesh',
  'region/d2/mesh',
] as const)

export type ManifoldPlanCarrier = typeof MANIFOLD_PLAN_CARRIERS[number]

const MANIFOLD_PLAN_PAYLOAD = Symbol('ManifoldPlanPayload')

/**
 * Opaque plan value owned by one backend session. Geometry is deliberately
 * reachable only through the narrow inspection helper below; compiler/core/UI
 * code never receives a Manifold type.
 */
export interface ManifoldPlanPayload {
  readonly carrierKey: ManifoldPlanCarrier
  readonly [MANIFOLD_PLAN_PAYLOAD]: true
}

export interface ManifoldPlanPayloadFamily {
  readonly 'solid-set/d3/mesh': ManifoldPlanPayload
  readonly 'region/d2/mesh': ManifoldPlanPayload
}

interface OwnedManifoldPlanPayload extends ManifoldPlanPayload {
  readonly handle: ManifoldKernelHandle
  readonly kernel: ManifoldKernelOps
  readonly provenance: ManifoldPlanProvenanceRegistry
}

interface ManifoldPlanProvenanceRegistry {
  readonly nodeByOriginalId: Map<number, number>
}

interface PayloadEntry {
  readonly lease: SemanticBackendPayloadLease
  readonly payload: OwnedManifoldPlanPayload
  released: boolean
}

export type ManifoldPlanBackendErrorCode =
  | 'E_MANIFOLD_PLAN_BUSY'
  | 'E_MANIFOLD_PLAN_CONTRACT'
  | 'E_MANIFOLD_PLAN_LIFECYCLE'
  | 'E_MANIFOLD_PLAN_UNSUPPORTED'

export class ManifoldPlanBackendError extends Error {
  constructor(
    readonly code: ManifoldPlanBackendErrorCode,
    message: string,
  ) {
    super(message)
    this.name = 'ManifoldPlanBackendError'
  }
}

export interface ManifoldPlanPayloadView {
  readonly carrierKey: ManifoldPlanCarrier
  readonly dimension: 2 | 3
  /** Snapshot; callers cannot mutate the session registry. */
  readonly nodeByOriginalId: ReadonlyMap<number, number>
}

/** @internal Mesh analysis/shadow qualification boundary, never a compiler API. */
export function inspectManifoldPlanPayload(payload: ManifoldPlanPayload): ManifoldPlanPayloadView {
  const owned = payload as Partial<OwnedManifoldPlanPayload>
  if (owned[MANIFOLD_PLAN_PAYLOAD] !== true
    || owned.handle === undefined
    || owned.kernel === undefined
    || owned.provenance === undefined) {
    throw new TypeError('Value is not a Manifold plan payload')
  }
  return Object.freeze({
    carrierKey: owned.carrierKey as ManifoldPlanCarrier,
    dimension: owned.carrierKey === 'region/d2/mesh' ? 2 : 3,
    nodeByOriginalId: new Map(owned.provenance.nodeByOriginalId),
  })
}

/** @internal The only allocating mesh-analysis operation exposed to assemblers. */
export function analyzeManifoldPlanSolid(
  payload: ManifoldPlanPayload,
): ManifoldKernelSolidAnalysis {
  const owned = payload as Partial<OwnedManifoldPlanPayload>
  if (owned[MANIFOLD_PLAN_PAYLOAD] !== true
    || owned.handle === undefined
    || owned.kernel === undefined
    || owned.carrierKey !== 'solid-set/d3/mesh') {
    throw new TypeError('Value is not a solid Manifold plan payload')
  }
  return owned.kernel.analyzeSolid(owned.handle)
}

function carrierKey(valueType: SemanticValueType): SemanticCarrierKey {
  return `${valueType.geometryKind}/${valueType.space}/${valueType.representation}`
}

function isSupportedCarrier(value: SemanticCarrierKey): value is ManifoldPlanCarrier {
  return (MANIFOLD_PLAN_CARRIERS as readonly string[]).includes(value)
}

function backendError(
  code: ManifoldPlanBackendErrorCode,
  message: string,
): ManifoldPlanBackendError {
  return new ManifoldPlanBackendError(code, message)
}

function safeDelete(
  kernel: ManifoldKernelOps,
  handle: ManifoldKernelHandle,
): unknown | undefined {
  try {
    kernel.delete(handle)
    return undefined
  } catch (error) {
    return error
  }
}

function requirePayload(
  input: SemanticRuntimeValue<ManifoldPlanPayloadFamily> | undefined,
  expectedDimension: 2 | 3,
  node: SemanticNode,
): OwnedManifoldPlanPayload {
  if (input === undefined || input.tag === 'empty') {
    throw backendError('E_MANIFOLD_PLAN_CONTRACT', `${node.kind} received a missing input after shared empty reduction`)
  }
  const payload = input.payload as Partial<OwnedManifoldPlanPayload>
  if (payload[MANIFOLD_PLAN_PAYLOAD] !== true
    || payload.handle === undefined
    || payload.kernel === undefined) {
    throw backendError('E_MANIFOLD_PLAN_CONTRACT', `${node.kind} received a foreign Manifold payload`)
  }
  const dimension = payload.carrierKey === 'region/d2/mesh' ? 2 : 3
  if (dimension !== expectedDimension) {
    throw backendError('E_MANIFOLD_PLAN_CONTRACT', `${node.kind} received a ${dimension}D input where ${expectedDimension}D was required`)
  }
  return payload as OwnedManifoldPlanPayload
}

function matrix3FromMatrix4(matrix: readonly number[]): readonly number[] {
  return [
    matrix[0], matrix[1], 0,
    matrix[4], matrix[5], 0,
    matrix[12], matrix[13], 1,
  ]
}

class ManifoldPlanSession implements SemanticBackendSession<ManifoldPlanPayloadFamily> {
  private readonly entries = new Map<SemanticBackendPayloadLease, PayloadEntry>()
  private readonly provenance: ManifoldPlanProvenanceRegistry = { nodeByOriginalId: new Map() }
  private closePromise: Promise<SemanticBackendCloseResult> | undefined
  private closing = false
  private inEvaluate = 0
  private quarantined = false
  private quarantineCause: unknown = undefined

  constructor(
    private readonly kernel: ManifoldKernelOps,
    private readonly signal: AbortSignal,
    private readonly releaseSession: (quarantined: boolean, cause?: unknown) => void,
  ) {}

  private quarantine(cause: unknown): void {
    if (this.quarantined) return
    this.quarantined = true
    this.quarantineCause = cause
  }

  private finishSession(): void {
    this.releaseSession(this.quarantined, this.quarantineCause)
  }

  validatePayload(
    expectedCarrier: SemanticCarrierKey,
    payload: unknown,
    lease: SemanticBackendPayloadLease,
  ): boolean {
    const entry = this.entries.get(lease)
    return entry !== undefined
      && !entry.released
      && entry.payload === payload
      && entry.payload.carrierKey === expectedCarrier
  }

  evaluate(
    node: SemanticNode,
    inputs: readonly SemanticRuntimeValue<ManifoldPlanPayloadFamily>[],
    context: SemanticBackendContext,
  ): SemanticBackendEvaluation<ManifoldPlanPayloadFamily> {
    if (this.closing) throw backendError('E_MANIFOLD_PLAN_LIFECYCLE', 'Cannot evaluate after session close has begun')
    if (this.signal.aborted || context.signal.aborted) {
      throw backendError('E_MANIFOLD_PLAN_LIFECYCLE', 'Manifold plan evaluation was cancelled')
    }
    if (context.languageContract !== 'legacy/current') {
      throw backendError('E_MANIFOLD_PLAN_UNSUPPORTED', 'ManifoldPlanBackend accepts legacy/current programs only')
    }
    const expectedCarrier = carrierKey(node.valueType)
    if (context.carrierKey !== expectedCarrier) {
      throw backendError('E_MANIFOLD_PLAN_CONTRACT', `Executor carrier does not match node carrier ${expectedCarrier}`)
    }
    if (!isSupportedCarrier(expectedCarrier)) {
      throw backendError('E_MANIFOLD_PLAN_UNSUPPORTED', `Manifold plan does not support carrier ${expectedCarrier}`)
    }
    if (node.valueType.evidence.tag !== 'representation-preserving') {
      throw backendError('E_MANIFOLD_PLAN_UNSUPPORTED', 'Manifold cannot verify certified approximation evidence')
    }

    this.inEvaluate++
    let handle: ManifoldKernelHandle | undefined
    try {
      handle = this.evaluateGeometry(node, inputs, context)
      if (this.signal.aborted || context.signal.aborted) {
        throw backendError('E_MANIFOLD_PLAN_LIFECYCLE', 'Manifold plan evaluation was cancelled')
      }
      if (this.shouldPromoteOriginal(node)) {
        const unpromoted = handle
        handle = undefined
        handle = this.promoteOriginal(unpromoted, context.nodeIndex)
      }
      // Empty Manifold objects are still authored legacy Shape records. Keep
      // their lease and handle through downstream operations: removing an
      // empty operand can change Manifold's exact ordering and provenance even
      // when the mathematical result is unchanged. The assembler alone omits
      // a materialized-empty root from publication.
      const result = this.allocate(
        node,
        expectedCarrier,
        handle,
        this.kernel.isEmpty(handle) ? 'materialized-empty' : 'value',
      )
      handle = undefined
      return result
    } catch (error) {
      if (handle !== undefined) {
        const cleanupError = safeDelete(this.kernel, handle)
        if (cleanupError !== undefined) {
          const failure = new AggregateError([error, cleanupError], `Manifold node ${node.id} failed and cleanup also failed`)
          this.quarantine(failure)
          throw failure
        }
      }
      if (error instanceof AggregateError) this.quarantine(error)
      throw error
    } finally {
      this.inEvaluate--
    }
  }

  releasePayload(lease: SemanticBackendPayloadLease): Promise<void> {
    const entry = this.entries.get(lease)
    if (entry === undefined || entry.released) return Promise.resolve()
    entry.released = true
    this.entries.delete(lease)
    const cleanupError = safeDelete(entry.payload.kernel, entry.payload.handle)
    if (cleanupError === undefined) return Promise.resolve()
    this.quarantine(cleanupError)
    return Promise.reject(cleanupError)
  }

  close(outcome: SemanticBackendCloseOutcome): Promise<SemanticBackendCloseResult> {
    if (this.closePromise !== undefined) return this.closePromise
    this.closing = true
    this.closePromise = Promise.resolve().then(() => {
      if (this.inEvaluate !== 0) {
        throw backendError('E_MANIFOLD_PLAN_LIFECYCLE', 'Session close is not quiescent')
      }
      if (outcome.tag !== 'commit') {
        const cleanupErrors = this.releaseAll(new Set())
        if (cleanupErrors.length > 0) {
          const failure = new AggregateError(cleanupErrors, 'Manifold plan abort cleanup failed')
          this.quarantine(failure)
          this.finishSession()
          throw failure
        }
        this.finishSession()
        return Object.freeze({ tag: 'closed' as const })
      }

      const retained = new Set(outcome.retained)
      if (retained.size !== outcome.retained.length
        || [...retained].some(lease => {
          const entry = this.entries.get(lease)
          return entry === undefined || entry.released
        })) {
        const cleanupErrors = this.releaseAll(new Set())
        const contractFailure = backendError(
          'E_MANIFOLD_PLAN_CONTRACT',
          'Commit retained a duplicate, foreign, or already released payload lease',
        )
        if (cleanupErrors.length > 0) {
          const failure = new AggregateError([contractFailure, ...cleanupErrors], 'Invalid Manifold commit and cleanup failure')
          this.quarantine(failure)
          this.finishSession()
          throw failure
        }
        this.finishSession()
        throw contractFailure
      }

      const cleanupErrors = this.releaseAll(retained)
      if (cleanupErrors.length > 0) {
        const retainedCleanupErrors = this.releaseAll(new Set())
        const failure = new AggregateError(
          [...cleanupErrors, ...retainedCleanupErrors],
          'Manifold plan commit cleanup failed',
        )
        this.quarantine(failure)
        this.finishSession()
        throw failure
      }
      return Object.freeze({
        tag: 'committed' as const,
        resultLease: this.resultLease(retained),
      })
    })
    return this.closePromise
  }

  private evaluateGeometry(
    node: SemanticNode,
    inputs: readonly SemanticRuntimeValue<ManifoldPlanPayloadFamily>[],
    context: SemanticBackendContext,
  ): ManifoldKernelHandle {
    switch (node.kind) {
      case 'box':
        return this.kernel.box(node.size, node.center)
      case 'sphere-polygonal':
        return this.kernel.sphere(node.radius, node.radialSegments)
      case 'cylinder-polygonal':
        return this.cylinder(node.height, node.radiusBottom, node.radiusTop, node.radialSegments, node.center)
      case 'polyhedron':
        return this.kernel.polyhedron(node.vertices, node.triangles)
      case 'rectangle':
        return this.kernel.rectangle(node.size, node.center)
      case 'circle-polygonal':
        return this.kernel.circle(node.radius, node.radialSegments)
      case 'polygon':
        return this.kernel.polygon(node.rings)
      case 'transform': {
        const producerName = context.producer?.operationName
        if (producerName === undefined || ![
          'translate', 'rotate', 'scale', 'mirror', 'multmatrix',
        ].includes(producerName)) {
          throw backendError(
            'E_MANIFOLD_PLAN_CONTRACT',
            'Transform evaluation requires one executor-attested transform producer',
          )
        }
        if (node.valueType.space === 'd2') {
          const input = requirePayload(inputs[0], 2, node)
          const determinant = node.matrix[0] * node.matrix[5] - node.matrix[4] * node.matrix[1]
          if (producerName === 'mirror' && determinant === 0) {
            return this.kernel.empty2()
          }
          return this.kernel.transform2(input.handle, matrix3FromMatrix4(node.matrix))
        }
        const input = requirePayload(inputs[0], 3, node)
        const determinant = node.matrix[0] * (node.matrix[5] * node.matrix[10] - node.matrix[9] * node.matrix[6])
          - node.matrix[4] * (node.matrix[1] * node.matrix[10] - node.matrix[9] * node.matrix[2])
          + node.matrix[8] * (node.matrix[1] * node.matrix[6] - node.matrix[5] * node.matrix[2])
        if (producerName === 'mirror' && determinant === 0) {
          return this.kernel.empty3()
        }
        return this.kernel.transform3(input.handle, node.matrix)
      }
      case 'boolean':
        return this.boolean(node.operation, node.valueType.space === 'd2' ? 2 : 3, inputs, node)
      case 'hull':
        return this.hull(node.valueType.space === 'd2' ? 2 : 3, inputs, node)
      case 'linear-extrude': {
        const input = requirePayload(inputs[0], 2, node)
        return this.kernel.linearExtrude(
          input.handle,
          node.height,
          node.slices,
          node.twistDegrees,
          node.scale,
          node.center,
        )
      }
      case 'rotate-extrude-polygonal': {
        const input = requirePayload(inputs[0], 2, node)
        return this.kernel.rotateExtrude(input.handle, node.radialSegments, node.angleDegrees)
      }
      case 'projection': {
        const input = requirePayload(inputs[0], 3, node)
        return this.kernel.projection(input.handle, node.cut)
      }
      case 'offset': {
        const input = requirePayload(inputs[0], 2, node)
        return this.kernel.offset(input.handle, node.distance)
      }
      case 'sphere-analytic':
      case 'cylinder-analytic':
      case 'circle-analytic':
      case 'rotate-extrude-analytic':
        throw backendError(
          'E_MANIFOLD_PLAN_UNSUPPORTED',
          `${node.kind} is an analytic B-rep operation and cannot be executed by the mesh-only Manifold backend`,
        )
    }
  }

  private cylinder(
    height: number,
    radiusBottom: number,
    radiusTop: number,
    radialSegments: number,
    center: boolean,
  ): ManifoldKernelHandle {
    if (radiusBottom > 0) {
      return this.kernel.cylinder(height, radiusBottom, radiusTop, radialSegments, center)
    }
    let current = this.kernel.cylinder(height, radiusTop, 0, radialSegments, center)
    try {
      let next = this.kernel.mirrorZ(current)
      const mirrorCleanup = safeDelete(this.kernel, current)
      current = next
      if (mirrorCleanup !== undefined) {
        throw new AggregateError([mirrorCleanup], 'Cylinder mirror cleanup failed')
      }
      if (!center) {
        next = this.kernel.translateZ(current, height)
        const translateCleanup = safeDelete(this.kernel, current)
        current = next
        if (translateCleanup !== undefined) {
          throw new AggregateError([translateCleanup], 'Cylinder translation cleanup failed')
        }
      }
      return current
    } catch (error) {
      const cleanupError = safeDelete(this.kernel, current)
      if (cleanupError !== undefined) {
        throw new AggregateError([error, cleanupError], 'Cylinder construction and cleanup failed')
      }
      throw error
    }
  }

  private boolean(
    operation: 'union' | 'intersection' | 'difference',
    dimension: 2 | 3,
    inputs: readonly SemanticRuntimeValue<ManifoldPlanPayloadFamily>[],
    node: SemanticNode,
  ): ManifoldKernelHandle {
    if (inputs.length === 0) {
      throw backendError('E_MANIFOLD_PLAN_CONTRACT', `${operation} received no values after shared empty reduction`)
    }
    return dimension === 2
      ? this.kernel.boolean2(operation, inputs.map(input => requirePayload(input, 2, node).handle))
      : this.kernel.boolean3(operation, inputs.map(input => requirePayload(input, 3, node).handle))
  }

  private hull(
    dimension: 2 | 3,
    inputs: readonly SemanticRuntimeValue<ManifoldPlanPayloadFamily>[],
    node: SemanticNode,
  ): ManifoldKernelHandle {
    if (inputs.length === 0) {
      throw backendError('E_MANIFOLD_PLAN_CONTRACT', 'hull received no values after shared empty reduction')
    }
    return dimension === 2
      ? this.kernel.hull2(inputs.map(input => requirePayload(input, 2, node).handle))
      : this.kernel.hull3(inputs.map(input => requirePayload(input, 3, node).handle))
  }

  private shouldPromoteOriginal(node: SemanticNode): boolean {
    return node.kind === 'box'
      || node.kind === 'sphere-polygonal'
      || node.kind === 'cylinder-polygonal'
      || node.kind === 'polyhedron'
      || node.kind === 'hull'
      || node.kind === 'linear-extrude'
      || node.kind === 'rotate-extrude-polygonal'
  }

  private promoteOriginal(handle: ManifoldKernelHandle, nodeIndex: number): ManifoldKernelHandle {
    if (handle.dimension !== 3) return handle
    let solid = handle
    try {
      if ((this.kernel.originalId(solid) ?? -1) < 0) {
        const promoted = this.kernel.asOriginal(solid)
        const cleanupError = safeDelete(this.kernel, solid)
        solid = promoted
        if (cleanupError !== undefined) {
          throw new AggregateError([cleanupError], 'Original promotion replacement cleanup failed')
        }
      }
      const originalId = this.kernel.originalId(solid) ?? -1
      if (originalId >= 0 && !this.provenance.nodeByOriginalId.has(originalId)) {
        this.provenance.nodeByOriginalId.set(originalId, nodeIndex)
      }
      return solid
    } catch (error) {
      const cleanupError = safeDelete(this.kernel, solid)
      if (cleanupError !== undefined) {
        throw new AggregateError([error, cleanupError], 'Original promotion and cleanup failed')
      }
      throw error
    }
  }

  private allocate(
    node: SemanticNode,
    expectedCarrier: ManifoldPlanCarrier,
    handle: ManifoldKernelHandle,
    tag: 'value' | 'materialized-empty',
  ): SemanticBackendEvaluation<ManifoldPlanPayloadFamily> {
    const lease = Object.freeze({}) as SemanticBackendPayloadLease
    const payload = Object.freeze({
      carrierKey: expectedCarrier,
      handle,
      kernel: this.kernel,
      provenance: this.provenance,
      [MANIFOLD_PLAN_PAYLOAD]: true as const,
    })
    this.entries.set(lease, { lease, payload, released: false })
    return Object.freeze({
      tag,
      valueType: node.valueType,
      evidence: PRESERVING_EVIDENCE,
      payload,
      lease,
    }) as SemanticBackendEvaluation<ManifoldPlanPayloadFamily>
  }

  private releaseAll(retained: ReadonlySet<SemanticBackendPayloadLease>): unknown[] {
    const cleanupErrors: unknown[] = []
    for (const [lease, entry] of [...this.entries]) {
      if (retained.has(lease)) continue
      entry.released = true
      this.entries.delete(lease)
      const cleanupError = safeDelete(entry.payload.kernel, entry.payload.handle)
      if (cleanupError !== undefined) cleanupErrors.push(cleanupError)
    }
    return cleanupErrors
  }

  private resultLease(retained: ReadonlySet<SemanticBackendPayloadLease>): SemanticBackendResultLease {
    let disposePromise: Promise<void> | undefined
    return Object.freeze({
      dispose: () => {
        if (disposePromise === undefined) {
          disposePromise = Promise.resolve().then(() => {
            const cleanupErrors = this.releaseAll(new Set())
            if (cleanupErrors.length > 0) {
              const failure = new AggregateError(cleanupErrors, 'Manifold result cleanup failed')
              this.quarantine(failure)
              this.finishSession()
              throw failure
            }
            this.finishSession()
          })
        }
        return disposePromise
      },
    })
  }
}

/**
 * Synchronous SemanticProgram backend over an already initialized, raw
 * Manifold module. One execution session may be evaluating at a time; committed
 * result leases may outlive the session until their explicit disposal.
 */
export class ManifoldPlanBackend implements SemanticProgramBackend<ManifoldPlanPayloadFamily> {
  private sessionOpen = false
  private quarantined = false
  private quarantineCause: unknown = undefined

  constructor(readonly kernel: ManifoldKernelOps) {}

  /**
   * Qualification-lane health only. A quarantined module instance must be
   * retired as a whole; callers may never probe it by attempting another
   * allocation.
   */
  get lifecycleState(): 'available' | 'busy' | 'quarantined' {
    if (this.quarantined) return 'quarantined'
    return this.sessionOpen ? 'busy' : 'available'
  }

  begin(context: SemanticBackendBeginContext): SemanticBackendSession<ManifoldPlanPayloadFamily> {
    if (context.languageContract !== 'legacy/current') {
      throw backendError('E_MANIFOLD_PLAN_UNSUPPORTED', 'ManifoldPlanBackend accepts legacy/current programs only')
    }
    if (this.quarantined) {
      const failure = backendError(
        'E_MANIFOLD_PLAN_LIFECYCLE',
        'ManifoldPlanBackend is quarantined after an indeterminate cleanup failure',
      )
      ;(failure as Error & { cause?: unknown }).cause = this.quarantineCause
      throw failure
    }
    if (this.sessionOpen) {
      throw backendError('E_MANIFOLD_PLAN_BUSY', 'ManifoldPlanBackend already has an open execution session')
    }
    this.sessionOpen = true
    let released = false
    return new ManifoldPlanSession(this.kernel, context.signal, (quarantined, cause) => {
      if (released) return
      released = true
      this.sessionOpen = false
      if (quarantined) {
        this.quarantined = true
        this.quarantineCause = cause
      }
    })
  }
}

/** Load a non-instrumented module so ownership stays per lease, not global. */
export async function loadManifoldPlanBackend(): Promise<ManifoldPlanBackend> {
  return new ManifoldPlanBackend(await loadManifoldKernelOps())
}
