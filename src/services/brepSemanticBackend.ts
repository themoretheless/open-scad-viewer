import {BrepSemanticBackendError, type BrepSemanticBackendErrorCode} from './brepSemanticErrors'
export {BrepSemanticBackendError, type BrepSemanticBackendErrorCode} from './brepSemanticErrors'
import {
  type SemanticNode,
  type SemanticValueType,
} from '../core/semanticProgram'
import { MAX_NATIVE_GEOMETRY_CHARACTERS } from '../core/nativeGeometry'
import type {NurbsBrep} from './geometry/brep'
import {GeometryKernelError,callGeometryRust} from './geometry/kernel'
import type {BrepProfile} from './geometry/brepProfile'
import type {
  SemanticBackendBeginContext,
  SemanticBackendCloseOutcome,
  SemanticBackendCloseResult,
  SemanticBackendContext,
  SemanticBackendEvaluation,
  SemanticBackendPayloadLease,
  SemanticBackendSession,
  SemanticCarrierKey,
  SemanticProgramBackend,
  SemanticRuntimeValue,
} from './semanticProgramExecutor'

const PRESERVING_EVIDENCE = Object.freeze({ tag: 'representation-preserving' as const })

export const BREP_SEMANTIC_CARRIERS = Object.freeze([
  'solid/d3/analytic-brep',
  'solid-set/d3/analytic-brep',
  'region/d2/analytic-brep',
] as const)
export type BrepSemanticCarrier = typeof BREP_SEMANTIC_CARRIERS[number]
const BREP_SEMANTIC_PAYLOAD = Symbol('BrepSemanticPayload')

/** A session-owned value; inspect through the checked snapshot boundary below. */
export interface BrepSemanticPayload {
  readonly carrierKey: BrepSemanticCarrier
  readonly [BREP_SEMANTIC_PAYLOAD]: true
}

export interface BrepSemanticPayloadFamily {
  readonly 'solid/d3/analytic-brep': BrepSemanticPayload
  readonly 'solid-set/d3/analytic-brep': BrepSemanticPayload
  readonly 'region/d2/analytic-brep': BrepSemanticPayload
}

export interface BrepSemanticPayloadView {
  readonly carrierKey: BrepSemanticCarrier
  /** Deeply frozen authored geometry, never a tessellated approximation. */
  readonly model: NurbsBrep
  /** Producing core node, not a claim about the origin of every Boolean face. */
  readonly nodeIndex: number
}

export interface BrepSemanticProfilePayloadView {
  readonly carrierKey: 'region/d2/analytic-brep'
  /** Deeply frozen retained lines/circular arcs with material-left orientation. */
  readonly profile: BrepProfile
  readonly nodeIndex: number
}

type BrepSemanticGeometry =
  | Readonly<{ kind: 'solid'; model: NurbsBrep }>
  | Readonly<{ kind: 'profile'; profile: BrepProfile }>

interface PayloadEntry {
  readonly payload: BrepSemanticPayload
  readonly nodeIndex: number
  readonly nativeLease: string
  geometry: BrepSemanticGeometry | undefined
}
const payloadEntries = new WeakMap<BrepSemanticPayload, PayloadEntry>()

export interface BrepSemanticBackendOptions {
  /** May lower, but never raise, the existing 4 MiB native snapshot bound. */
  readonly maxRetainedGeometryCharacters?: number
}

function failure(code: BrepSemanticBackendErrorCode, message: string): BrepSemanticBackendError {
  return new BrepSemanticBackendError(code, message)
}

/** The assembler may retain this immutable snapshot; disposed payloads cannot be inspected anew. */
export function inspectBrepSemanticPayload(payload: BrepSemanticPayload): BrepSemanticPayloadView {
  const entry = payloadEntries.get(payload)
  if (entry === undefined) throw failure('E_BREP_SEMANTIC_CONTRACT', 'Foreign B-rep semantic payload')
  if (entry.geometry === undefined) throw failure('E_BREP_SEMANTIC_LIFECYCLE', 'B-rep semantic payload has been released')
  if (entry.geometry.kind !== 'solid') throw failure('E_BREP_SEMANTIC_UNSUPPORTED', 'A 2D B-rep profile requires extrusion before solid display')
  return Object.freeze({ carrierKey: payload.carrierKey, model: entry.geometry.model, nodeIndex: entry.nodeIndex })
}

export function inspectBrepSemanticProfilePayload(payload: BrepSemanticPayload): BrepSemanticProfilePayloadView {
  const entry = payloadEntries.get(payload)
  if (entry === undefined) throw failure('E_BREP_SEMANTIC_CONTRACT', 'Foreign B-rep semantic payload')
  if (entry.geometry === undefined) throw failure('E_BREP_SEMANTIC_LIFECYCLE', 'B-rep semantic payload has been released')
  if (entry.geometry.kind !== 'profile') throw failure('E_BREP_SEMANTIC_CONTRACT', 'A solid B-rep is not a planar profile')
  return Object.freeze({ carrierKey: 'region/d2/analytic-brep', profile: entry.geometry.profile, nodeIndex: entry.nodeIndex })
}

function carrierKey(valueType: SemanticValueType): SemanticCarrierKey {
  return `${valueType.geometryKind}/${valueType.space}/${valueType.representation}`
}

function freezeGeometry<T>(value: T): T {
  if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) freezeGeometry(child)
    Object.freeze(value)
  }
  return value
}

class BrepSemanticSession implements SemanticBackendSession<BrepSemanticPayloadFamily> {
  private readonly entries = new Map<SemanticBackendPayloadLease, PayloadEntry>()
  private readonly issued = new WeakSet<SemanticBackendPayloadLease>()
  private readonly localPayloads = new WeakSet<BrepSemanticPayload>()
  private closing = false
  private failed = false
  private closePromise: Promise<SemanticBackendCloseResult> | undefined
  private nativeSession: string | undefined
  private nativeCommitted = false

  constructor(
    private readonly context: SemanticBackendBeginContext,
    private readonly maxRetainedGeometryCharacters: number,
  ) {}

  validatePayload(carrier: SemanticCarrierKey, payload: unknown, lease: SemanticBackendPayloadLease): boolean {
    const entry = this.entries.get(lease)
    return !this.closing && !this.failed && entry?.geometry !== undefined
      && entry.payload === payload && entry.payload.carrierKey === carrier
  }

  evaluate(
    node: SemanticNode,
    inputs: readonly SemanticRuntimeValue<BrepSemanticPayloadFamily>[],
    context: SemanticBackendContext,
  ): SemanticBackendEvaluation<BrepSemanticPayloadFamily> {
    this.requireOpen(context)
    try {
      const key = carrierKey(node.valueType)
      if (context.programHash !== this.context.programHash
        || context.languageContract !== this.context.languageContract
        || context.carrierKey !== key || context.nodeIndex !== node.id) {
        throw failure('E_BREP_SEMANTIC_CONTRACT', 'B-rep node does not belong to this execution context')
      }
      this.checkInputReferences(inputs, context)
      const native = this.evaluateGeometry(node, inputs, context)
      if (native.tag === 'empty') {
        this.requireOpen(context)
        this.nativeCall({action:'release',lease:native.lease})
        return Object.freeze({tag:'empty',valueType:node.valueType,evidence:PRESERVING_EVIDENCE})
      }
      const geometry = native.geometry
      this.requireOpen(context)
      const payload: BrepSemanticPayload = Object.freeze({ carrierKey: key as BrepSemanticCarrier, [BREP_SEMANTIC_PAYLOAD]: true as const })
      const lease = Object.freeze({}) as SemanticBackendPayloadLease<BrepSemanticCarrier>
      const entry: PayloadEntry = { payload, nativeLease:native.lease, nodeIndex: node.id, geometry: freezeGeometry(geometry) }
      this.entries.set(lease, entry)
      this.issued.add(lease)
      this.localPayloads.add(payload)
      payloadEntries.set(payload, entry)
      return Object.freeze({ tag: 'value', valueType: node.valueType, evidence: PRESERVING_EVIDENCE, payload, lease }) as SemanticBackendEvaluation<BrepSemanticPayloadFamily>
    } catch (error) {
      this.failed = true
      this.disposeNative()
      if (error instanceof BrepSemanticBackendError) throw error
      throw new BrepSemanticBackendError(
        error instanceof GeometryKernelError && error.code.startsWith('BREP_UNSUPPORTED_')
          ? 'E_BREP_SEMANTIC_UNSUPPORTED' : error instanceof GeometryKernelError && error.code === 'BREP_SEMANTIC_BUDGET' ? 'E_BREP_SEMANTIC_BUDGET' : error instanceof GeometryKernelError && error.code === 'BREP_SEMANTIC_CONTRACT' ? 'E_BREP_SEMANTIC_CONTRACT' : 'E_BREP_SEMANTIC_KERNEL',
        `${node.kind}: ${error instanceof Error ? error.message : 'B-rep evaluation failed'}`,
        error,
      )
    }
  }

  releasePayload(lease: SemanticBackendPayloadLease): Promise<void> {
    if (!this.issued.has(lease)) {
      return Promise.reject(failure('E_BREP_SEMANTIC_CONTRACT', 'Cannot release a foreign B-rep payload lease'))
    }
    if (!this.entries.has(lease)) return Promise.resolve()
    if (this.closing) {
      return Promise.reject(failure('E_BREP_SEMANTIC_LIFECYCLE', 'Payload ownership has passed to session close'))
    }
    this.release(lease)
    return Promise.resolve()
  }

  close(outcome: SemanticBackendCloseOutcome): Promise<SemanticBackendCloseResult> {
    if (this.closePromise !== undefined) return this.closePromise
    this.closing = true
    // Capture commit intent synchronously: callers cannot mutate retained while close is pending.
    const retained = outcome.tag === 'commit' ? [...outcome.retained] : null
    this.closePromise = Promise.resolve().then(() => {
      if (retained === null) {
        this.releaseAll()
        return Object.freeze({ tag: 'closed' as const })
      }
      const retainedSet = new Set(retained)
      if (this.failed || this.context.signal.aborted || retainedSet.size !== retained.length
        || retained.some(lease => !this.entries.has(lease))) {
        this.releaseAll()
        throw failure('E_BREP_SEMANTIC_CONTRACT', 'Cannot commit a failed/cancelled session or duplicate, foreign, or released payload leases')
      }
      for (const lease of this.entries.keys()) if (!retainedSet.has(lease)) this.release(lease)
      try {
        if(this.nativeSession!==undefined) this.nativeCall({action:'commit',retained:retained.map(lease=>this.entries.get(lease)!.nativeLease)})
        this.nativeCommitted=true
      } catch(error) { this.releaseAll(); throw error }
      let disposePromise: Promise<void> | undefined
      return Object.freeze({
        tag: 'committed' as const,
        resultLease: Object.freeze({
          dispose: () => {
            disposePromise ??= Promise.resolve().then(() => this.releaseAll())
            return disposePromise
          },
        }),
      })
    })
    return this.closePromise
  }

  private requireOpen(context: SemanticBackendContext): void {
    if (this.closing || this.failed || this.context.signal.aborted || context.signal.aborted) {
      throw failure('E_BREP_SEMANTIC_LIFECYCLE', 'B-rep semantic session is closed, failed, or cancelled')
    }
  }

  private checkInputReferences(
    inputs: readonly SemanticRuntimeValue<BrepSemanticPayloadFamily>[],
    context: SemanticBackendContext,
  ): void {
    if (context.inputNodeIndices.length !== inputs.length) {
      throw failure('E_BREP_SEMANTIC_CONTRACT', 'B-rep input references do not match runtime inputs')
    }
    for (let index = 0; index < inputs.length; index++) {
      const reference = context.inputNodeIndices[index]
      const input = inputs[index]
      if (input.tag !== 'value' || !this.localPayloads.has(input.payload)) {
        throw failure('E_BREP_SEMANTIC_CONTRACT', 'B-rep received a foreign or unreduced empty input')
      }
      const entry = payloadEntries.get(input.payload)
      if (entry?.geometry === undefined || entry.nodeIndex !== reference
        || input.payload.carrierKey !== carrierKey(input.valueType)
        || input.evidence.tag !== 'representation-preserving') {
        throw failure('E_BREP_SEMANTIC_CONTRACT', 'B-rep input payload is stale or does not match its declared carrier/node')
      }
    }
  }

  private nativeCall<T=Record<string,unknown>>(args:Record<string,unknown>):T {
    return callGeometryRust('brep_session',{...args,session:this.nativeSession})
  }

  private disposeNative():void {
    const session=this.nativeSession
    this.nativeSession=undefined
    if(session!==undefined)callGeometryRust('brep_session',{action:'dispose',session})
  }

  private evaluateGeometry(node:SemanticNode,inputs:readonly SemanticRuntimeValue<BrepSemanticPayloadFamily>[],context:SemanticBackendContext):
    {tag:'empty';lease:string}|{tag:'value';lease:string;geometry:BrepSemanticGeometry} {
    if(this.nativeSession===undefined){
      this.nativeSession=callGeometryRust<{session:string}>('brep_session',{action:'begin',maxNodes:Math.max(1,this.context.limits.maxNodes),maxBytes:Math.max(65536,this.maxRetainedGeometryCharacters*2),maxCharacters:this.maxRetainedGeometryCharacters}).session
    }
    const leases=inputs.map(input=>{
      if(input.tag!=='value')throw failure('E_BREP_SEMANTIC_CONTRACT','Expected a non-empty reduced input')
      const entry=payloadEntries.get(input.payload)
      if(!entry?.geometry)throw failure('E_BREP_SEMANTIC_CONTRACT','Missing semantic input geometry')
      return entry.nativeLease
    })
    const {lease}=this.nativeCall<{lease:string}>({action:'evaluate',node,inputNodeIndices:context.inputNodeIndices,inputs:leases})
    const snapshot=this.nativeCall<{tag:'empty'}|{tag:'value';geometry:BrepSemanticGeometry}>({action:'snapshot',lease})
    return snapshot.tag==='empty'?{tag:'empty',lease}:{tag:'value',lease,geometry:snapshot.geometry}
  }

  private release(lease: SemanticBackendPayloadLease): void {
    const entry = this.entries.get(lease)
    if (entry !== undefined) {
      if(this.nativeSession!==undefined&&!this.nativeCommitted)this.nativeCall({action:'release',lease:entry.nativeLease})
      entry.geometry = undefined
    }
    this.entries.delete(lease)
  }

  private releaseAll(): void {
    this.disposeNative()
    for (const lease of this.entries.keys()) this.release(lease)
  }
}

/**
 * Analytic primitives, retained planar profiles, straight extrusion and bounded
 * Booleans over the own synchronous Rust WASM.
 * Use only behind executeSemanticProgram's trusted lowering boundary. It sees no
 * source text, display policy, routing manifest, or qualification certificates.
 * Rust owns session geometry; host records are immutable display snapshots.
 * Native handles are confined to the current WASM instance and disposed on close.
 */
export class BrepSemanticBackend implements SemanticProgramBackend<BrepSemanticPayloadFamily> {
  private readonly maxRetainedGeometryCharacters: number

  constructor(options: BrepSemanticBackendOptions = {}) {
    const limit = options.maxRetainedGeometryCharacters ?? MAX_NATIVE_GEOMETRY_CHARACTERS
    if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_NATIVE_GEOMETRY_CHARACTERS) {
      throw failure('E_BREP_SEMANTIC_BUDGET', 'B-rep retained geometry limit must be 1..4194304 characters')
    }
    this.maxRetainedGeometryCharacters = limit
  }

  begin(context: SemanticBackendBeginContext): SemanticBackendSession<BrepSemanticPayloadFamily> {
    if (context.languageContract !== 'openscad-viewer/brep-1') {
      throw failure('E_BREP_SEMANTIC_UNSUPPORTED', 'B-rep semantic backend requires openscad-viewer/brep-1')
    }
    if (context.signal.aborted) throw failure('E_BREP_SEMANTIC_LIFECYCLE', 'Cannot begin a cancelled B-rep session')
    if (!Number.isSafeInteger(context.limits.maxNodes) || context.limits.maxNodes < 0) {
      throw failure('E_BREP_SEMANTIC_CONTRACT', 'B-rep session requires a finite node budget')
    }
    return new BrepSemanticSession(Object.freeze({
      programHash: context.programHash,
      languageContract: context.languageContract,
      limits: Object.freeze({ maxNodes: context.limits.maxNodes }),
      signal: context.signal,
    }), this.maxRetainedGeometryCharacters)
  }
}

export function createBrepSemanticBackend(options: BrepSemanticBackendOptions = {}): BrepSemanticBackend {
  return new BrepSemanticBackend(options)
}
