import {
  semanticNodeInputs,
  semanticNodeProducerOperationNames,
  semanticResultItems,
  type SemanticColor,
  type SemanticGeometryKind,
  type SemanticNode,
  type SemanticOccurrence,
  type SemanticProgramV1,
  type SemanticRepresentation,
  type SemanticSpace,
  type SemanticValueType,
} from '../core/semanticProgram'
import type { SemanticProgramAttestation } from './semanticProgramCodec'
import {
  requireTrustedSemanticLowering,
  type SemanticLoweringSuccess,
} from './semanticProgramLowerer'
export { checkSemanticExecutionControl } from './semanticProgramExecutor/checkSemanticExecutionControl'
export { executeSemanticProgram } from './semanticProgramExecutor/executeSemanticProgram'

export type SemanticCarrierKey = `${SemanticGeometryKind}/${SemanticSpace}/${SemanticRepresentation}`

type SemanticPayloadKey<TPayloadFamily extends object> = keyof TPayloadFamily & SemanticCarrierKey

export type SemanticRuntimeValue<TPayloadFamily extends object> =
  | Readonly<{
    tag: 'empty'
    valueType: SemanticValueType
    evidence: SemanticRuntimeEvidence
  }>
  | {
    [K in SemanticPayloadKey<TPayloadFamily>]: Readonly<{
      tag: 'value' | 'materialized-empty'
      valueType: SemanticValueType
      evidence: SemanticRuntimeEvidence
      payload: TPayloadFamily[K]
    }>
  }[SemanticPayloadKey<TPayloadFamily>]

declare const SEMANTIC_BACKEND_PAYLOAD_LEASE: unique symbol

/** Provider-created, unique, session-local ownership token. */
export type SemanticBackendPayloadLease<K extends SemanticCarrierKey = SemanticCarrierKey> = Readonly<{
  [SEMANTIC_BACKEND_PAYLOAD_LEASE]: K
}>

export type SemanticBackendEvaluation<TPayloadFamily extends object> =
  | Readonly<{
    tag: 'empty'
    valueType: SemanticValueType
    evidence: SemanticRuntimeEvidence
  }>
  | {
    [K in SemanticPayloadKey<TPayloadFamily>]: Readonly<{
      tag: 'value' | 'materialized-empty'
      valueType: SemanticValueType
      evidence: SemanticRuntimeEvidence
      payload: TPayloadFamily[K]
      lease: SemanticBackendPayloadLease<K>
    }>
  }[SemanticPayloadKey<TPayloadFamily>]

export type SemanticRuntimeEvidence =
  | Readonly<{ tag: 'representation-preserving' }>
  | Readonly<{
    tag: 'certified-approximation'
    certificateProfile: string
    certificatePolicyHash: string
    certificateId: string
  }>

export interface SemanticBackendContext {
  readonly nodeIndex: number
  readonly carrierKey: SemanticCarrierKey
  /** Node references aligned one-for-one with the possibly empty-reduced inputs. */
  readonly inputNodeIndices: readonly number[]
  /** Core-only producer identity; never source text, spans, route, or envelope policy. */
  readonly producer: Readonly<{
    occurrenceIndex: number
    operationIndex: number
    operationName: string
  }> | null
  readonly programHash: string
  readonly languageContract: SemanticProgramV1['core']['language']['contract']
  readonly signal: AbortSignal
}

export interface SemanticBackendBeginContext {
  readonly programHash: string
  readonly languageContract: SemanticProgramV1['core']['language']['contract']
  readonly limits: Readonly<{ maxNodes: number }>
  readonly signal: AbortSignal
}

export type SemanticBackendCloseOutcome =
  | Readonly<{
    tag: 'commit'
    retained: readonly SemanticBackendPayloadLease[]
  }>
  | Readonly<{
    tag: 'abort'
    code: 'E_SEMANTIC_ABORTED' | 'E_SEMANTIC_DEADLINE'
    node: number | null
  }>
  | Readonly<{
    tag: 'failure'
    code: SemanticExecutionErrorCode
    node: number | null
  }>

export interface SemanticBackendResultLease {
  /** Must be concurrency-safe and idempotent. */
  dispose(): Promise<void>
}

export type SemanticBackendCloseResult =
  | Readonly<{ tag: 'closed' }>
  | Readonly<{ tag: 'committed'; resultLease: SemanticBackendResultLease }>

export interface SemanticBackendSession<TPayloadFamily extends object> {
  /** Provider-owned carrier check; opaque payloads are never inferred from truthiness. */
  validatePayload(
    carrierKey: SemanticCarrierKey,
    payload: unknown,
    lease: SemanticBackendPayloadLease,
  ): boolean
  evaluate(
    node: SemanticNode,
    inputs: readonly SemanticRuntimeValue<TPayloadFamily>[],
    context: SemanticBackendContext,
  ): SemanticBackendEvaluation<TPayloadFamily> | Promise<SemanticBackendEvaluation<TPayloadFamily>>
  /** Ordinary release is idempotent; close remains the cleanup backstop. */
  releasePayload(lease: SemanticBackendPayloadLease): Promise<void>
  /** Mandatory quiescence and ownership-transfer boundary. */
  close(outcome: SemanticBackendCloseOutcome): Promise<SemanticBackendCloseResult>
}

/** A kernel-neutral backend can see validated core nodes, never source or route choice. */
export interface SemanticProgramBackend<TPayloadFamily extends object> {
  /** Synchronous and atomic: a throw means no session was published. */
  begin(context: SemanticBackendBeginContext): SemanticBackendSession<TPayloadFamily>
}

export interface SemanticExecutionControl {
  readonly shouldAbort?: () => boolean
  readonly signal?: AbortSignal
  readonly now?: () => number
  readonly deadlineAt?: number
  readonly maxNodes?: number
  readonly onNode?: (completed: number, total: number) => void
}

export type SemanticExecutionErrorCode =
  | 'E_SEMANTIC_ABORTED'
  | 'E_SEMANTIC_DEADLINE'
  | 'E_SEMANTIC_BUDGET'
  | 'E_SEMANTIC_BACKEND_BEGIN'
  | 'E_SEMANTIC_BACKEND_FAILURE'
  | 'E_SEMANTIC_BACKEND_TYPE'
  | 'E_SEMANTIC_BACKEND_CLOSE'
  | 'E_SEMANTIC_LANGUAGE_TERMINAL'
  | 'E_SEMANTIC_CALLBACK'

export class SemanticProgramExecutionError extends Error {
  backendCause: unknown = undefined
  cleanupError: unknown = undefined
  terminalDiagnostic: number | null = null

  constructor(
    readonly code: SemanticExecutionErrorCode,
    readonly node: number | null,
    message: string,
  ) {
    super(message)
    this.name = 'SemanticProgramExecutionError'
  }
}

export interface SemanticExecutedOutput<TPayloadFamily extends object> {
  readonly value: SemanticRuntimeValue<TPayloadFamily>
  readonly occurrence: SemanticOccurrence
  readonly color: SemanticColor
}

export interface SemanticExecutionResult<TPayloadFamily extends object> {
  readonly program: SemanticProgramV1
  readonly attestation: SemanticProgramAttestation
  readonly outputs: readonly SemanticExecutedOutput<TPayloadFamily>[]
  readonly disposed: boolean
  dispose(): Promise<void>
  [Symbol.asyncDispose](): Promise<void>
}
