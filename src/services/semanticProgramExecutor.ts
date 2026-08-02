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

function semanticCarrierKey(valueType: SemanticValueType): SemanticCarrierKey {
  return `${valueType.geometryKind}/${valueType.space}/${valueType.representation}`
}

function checkControl(control: SemanticExecutionControl, node: number | null): void {
  if (control.signal?.aborted) throw new SemanticProgramExecutionError('E_SEMANTIC_ABORTED', node, 'Semantic program execution was cancelled')
  try {
    if (control.shouldAbort?.()) throw new SemanticProgramExecutionError('E_SEMANTIC_ABORTED', node, 'Semantic program execution was cancelled')
  } catch (error) {
    if (error instanceof SemanticProgramExecutionError) throw error
    throw new SemanticProgramExecutionError('E_SEMANTIC_ABORTED', node, 'Semantic execution cancellation callback failed')
  }
  if (control.deadlineAt !== undefined) {
    if (!Number.isFinite(control.deadlineAt)) throw new SemanticProgramExecutionError('E_SEMANTIC_DEADLINE', node, 'deadlineAt must be finite')
    const now = control.now ?? (() => performance.now())
    let sampled: number
    try {
      sampled = now()
    } catch {
      throw new SemanticProgramExecutionError('E_SEMANTIC_DEADLINE', node, 'Semantic execution clock callback failed')
    }
    if (!Number.isFinite(sampled)) throw new SemanticProgramExecutionError('E_SEMANTIC_DEADLINE', node, 'Semantic execution clock must return a finite number')
    if (sampled >= control.deadlineAt) throw new SemanticProgramExecutionError('E_SEMANTIC_DEADLINE', node, 'Semantic program execution deadline exceeded')
  }
}

async function awaitBackendWithControl<T>(
  start: () => T | Promise<T>,
  control: SemanticExecutionControl,
  controller: AbortController,
  node: number,
): Promise<T> {
  let timeout: ReturnType<typeof setTimeout> | undefined
  let cancellationPoll: ReturnType<typeof setInterval> | undefined
  let removeExternalAbort: (() => void) | undefined
  let stopped = false
  const interruption = new Promise<never>((_, reject) => {
    const stop = (error: SemanticProgramExecutionError) => {
      if (stopped) return
      stopped = true
      controller.abort(error)
      reject(error)
    }
    if (control.signal) {
      const onAbort = () => stop(new SemanticProgramExecutionError('E_SEMANTIC_ABORTED', node, 'Semantic program execution was cancelled'))
      control.signal.addEventListener('abort', onAbort, { once: true })
      removeExternalAbort = () => control.signal?.removeEventListener('abort', onAbort)
      if (control.signal.aborted) onAbort()
    }
    if (control.shouldAbort) {
      cancellationPoll = setInterval(() => {
        try {
          if (control.shouldAbort?.()) stop(new SemanticProgramExecutionError('E_SEMANTIC_ABORTED', node, 'Semantic program execution was cancelled'))
        } catch {
          stop(new SemanticProgramExecutionError('E_SEMANTIC_ABORTED', node, 'Semantic execution cancellation callback failed'))
        }
      }, 8)
    }
    if (control.deadlineAt !== undefined) {
      const now = control.now ?? (() => performance.now())
      let sampled: number
      try {
        sampled = now()
      } catch {
        stop(new SemanticProgramExecutionError('E_SEMANTIC_DEADLINE', node, 'Semantic execution clock callback failed'))
        return
      }
      if (!Number.isFinite(sampled)) {
        stop(new SemanticProgramExecutionError('E_SEMANTIC_DEADLINE', node, 'Semantic execution clock must return a finite number'))
        return
      }
      if (sampled >= control.deadlineAt) {
        stop(new SemanticProgramExecutionError('E_SEMANTIC_DEADLINE', node, 'Semantic program execution deadline exceeded'))
        return
      }
      timeout = setTimeout(() => stop(new SemanticProgramExecutionError(
        'E_SEMANTIC_DEADLINE', node, 'Semantic program execution deadline exceeded',
      )), Math.min(0x7fffffff, control.deadlineAt - sampled))
    }
  })
  const pending = Promise.resolve().then(() => stopped ? new Promise<T>(() => {}) : start())
  try {
    return await Promise.race([pending, interruption])
  } finally {
    if (timeout !== undefined) clearTimeout(timeout)
    if (cancellationPoll !== undefined) clearInterval(cancellationPoll)
    removeExternalAbort?.()
  }
}

function snapshotDataRecord(
  value: unknown,
  invalid: () => SemanticProgramExecutionError,
): ReadonlyMap<string, unknown> {
  try {
    if (value === null || typeof value !== 'object' || Array.isArray(value)) throw invalid()
    const prototype = Object.getPrototypeOf(value)
    if (prototype !== Object.prototype && prototype !== null) throw invalid()
    const keys = Reflect.ownKeys(value)
    if (keys.some((key): key is symbol => typeof key === 'symbol')) throw invalid()
    const snapshot = new Map<string, unknown>()
    for (const key of keys as string[]) {
      const descriptor = Object.getOwnPropertyDescriptor(value, key)
      if (descriptor === undefined || !descriptor.enumerable || !('value' in descriptor)) throw invalid()
      snapshot.set(key, descriptor.value)
    }
    return snapshot
  } catch (error) {
    if (error instanceof SemanticProgramExecutionError) throw error
    throw invalid()
  }
}

function requireExactKeys(
  record: ReadonlyMap<string, unknown>,
  expected: readonly string[],
  invalid: () => SemanticProgramExecutionError,
): void {
  if (record.size !== expected.length || expected.some(key => !record.has(key))) throw invalid()
}

function snapshotValueType(
  value: unknown,
  expected: SemanticValueType,
  invalid: () => SemanticProgramExecutionError,
): void {
  const record = snapshotDataRecord(value, invalid)
  requireExactKeys(record, ['geometryKind', 'space', 'representation', 'evidence'], invalid)
  if (record.get('geometryKind') !== expected.geometryKind
    || record.get('space') !== expected.space
    || record.get('representation') !== expected.representation) throw invalid()
  const evidence = snapshotDataRecord(record.get('evidence'), invalid)
  if (expected.evidence.tag === 'representation-preserving') {
    requireExactKeys(evidence, ['tag'], invalid)
    if (evidence.get('tag') !== 'representation-preserving') throw invalid()
    return
  }
  requireExactKeys(evidence, ['tag', 'certificateProfile', 'certificatePolicyHash'], invalid)
  if (evidence.get('tag') !== 'certified-approximation'
    || evidence.get('certificateProfile') !== expected.evidence.certificateProfile
    || evidence.get('certificatePolicyHash') !== expected.evidence.certificatePolicyHash) throw invalid()
}

function snapshotRuntimeValue<TPayloadFamily extends object>(
  value: unknown,
  expected: SemanticValueType,
  node: number,
  allowMaterializedEmpty: boolean,
  session: SemanticBackendSession<TPayloadFamily>,
  recordLease: (lease: SemanticBackendPayloadLease) => void,
): SemanticRuntimeValue<TPayloadFamily> {
  const invalid = () => new SemanticProgramExecutionError('E_SEMANTIC_BACKEND_TYPE', node, 'Backend returned a value incompatible with the node valueType/evidence contract')
  const record = snapshotDataRecord(value, invalid)
  const tag = record.get('tag')
  const required = tag === 'empty'
    ? ['tag', 'valueType', 'evidence']
    : tag === 'value' || tag === 'materialized-empty'
      ? ['tag', 'valueType', 'evidence', 'payload', 'lease'] : []
  if (required.length === 0) throw invalid()
  if (tag === 'materialized-empty' && !allowMaterializedEmpty) throw invalid()
  requireExactKeys(record, required, invalid)
  snapshotValueType(record.get('valueType'), expected, invalid)
  const evidence = snapshotDataRecord(record.get('evidence'), invalid)
  if (expected.evidence.tag === 'representation-preserving') {
    requireExactKeys(evidence, ['tag'], invalid)
    if (evidence.get('tag') !== 'representation-preserving') throw invalid()
    const frozenEvidence = Object.freeze({ tag: 'representation-preserving' as const })
    if (tag === 'empty') return Object.freeze({ tag: 'empty', valueType: expected, evidence: frozenEvidence })
    const payload = record.get('payload')
    if (payload === null || payload === undefined) throw invalid()
    const lease = record.get('lease')
    if (lease === null || (typeof lease !== 'object' && typeof lease !== 'function')) throw invalid()
    let validPayload = false
    try {
      validPayload = session.validatePayload(
        semanticCarrierKey(expected),
        payload,
        lease as SemanticBackendPayloadLease,
      ) === true
    } catch {
      throw invalid()
    }
    if (!validPayload) throw invalid()
    recordLease(lease as SemanticBackendPayloadLease)
    return Object.freeze({
      tag: tag as 'value' | 'materialized-empty', valueType: expected, evidence: frozenEvidence,
      payload,
    }) as SemanticRuntimeValue<TPayloadFamily>
  }
  // Certified values are reserved in the schema but fail closed until a
  // certificate byte store and profile verifier are injected here.
  throw invalid()
}

function typedEmpty<TPayloadFamily extends object>(
  node: SemanticNode,
): SemanticRuntimeValue<TPayloadFamily> {
  if (node.valueType.evidence.tag !== 'representation-preserving') {
    throw new SemanticProgramExecutionError('E_SEMANTIC_BACKEND_TYPE', node.id, 'Cannot synthesize certified emptiness without a trusted certificate verifier')
  }
  return Object.freeze({
    tag: 'empty' as const,
    valueType: node.valueType,
    evidence: Object.freeze({ tag: 'representation-preserving' as const }),
  })
}

function reduceEmptyInputs<TPayloadFamily extends object>(
  node: SemanticNode,
  inputs: readonly SemanticRuntimeValue<TPayloadFamily>[],
): Readonly<{
  result?: SemanticRuntimeValue<TPayloadFamily>
  inputs: readonly SemanticRuntimeValue<TPayloadFamily>[]
  positions: readonly number[]
}> {
  const allPositions = Object.freeze(inputs.map((_, index) => index))
  const select = (predicate: (input: SemanticRuntimeValue<TPayloadFamily>, index: number) => boolean) => {
    const positions = Object.freeze(allPositions.filter(index => predicate(inputs[index], index)))
    return Object.freeze({
      positions,
      inputs: Object.freeze(positions.map(index => inputs[index])),
    })
  }
  switch (node.kind) {
    case 'transform':
    case 'linear-extrude':
    case 'rotate-extrude-analytic':
    case 'rotate-extrude-polygonal':
    case 'projection':
    case 'offset':
      return inputs[0]?.tag === 'empty'
        ? { result: typedEmpty(node), inputs: Object.freeze([]), positions: Object.freeze([]) }
        : { inputs, positions: allPositions }
    case 'hull': {
      const present = select(input => input.tag !== 'empty')
      return present.inputs.length === 0 ? { result: typedEmpty(node), ...present } : present
    }
    case 'boolean': {
      if (node.operation === 'intersection') {
        return inputs.some(input => input.tag === 'empty')
          ? { result: typedEmpty(node), inputs: Object.freeze([]), positions: Object.freeze([]) }
          : { inputs, positions: allPositions }
      }
      if (node.operation === 'difference') {
        if (inputs[0]?.tag === 'empty') return { result: typedEmpty(node), inputs: Object.freeze([]), positions: Object.freeze([]) }
        return select((input, index) => index === 0 || input.tag !== 'empty')
      }
      const present = select(input => input.tag !== 'empty')
      return present.inputs.length === 0 ? { result: typedEmpty(node), ...present } : present
    }
    default:
      return { inputs, positions: allPositions }
  }
}

function nodeMustReturnValueAfterReduction(node: SemanticNode): boolean {
  if (node.kind === 'hull'
    || node.kind === 'linear-extrude'
    || node.kind === 'rotate-extrude-analytic'
    || node.kind === 'rotate-extrude-polygonal') return true
  if (node.kind === 'boolean') return node.operation === 'union'
  if (node.kind === 'projection') return !node.cut
  if (node.kind === 'offset') return node.distance >= 0
  return node.kind === 'box'
    || node.kind === 'sphere-analytic'
    || node.kind === 'sphere-polygonal'
    || node.kind === 'cylinder-analytic'
    || node.kind === 'cylinder-polygonal'
    || node.kind === 'rectangle'
    || node.kind === 'circle-analytic'
    || node.kind === 'circle-polygonal'
}

function executionFailure(error: unknown, node: number | null): SemanticProgramExecutionError {
  if (error instanceof SemanticProgramExecutionError) return error
  const failure = new SemanticProgramExecutionError(
    'E_SEMANTIC_BACKEND_FAILURE',
    node,
    'Semantic backend evaluation failed',
  )
  failure.backendCause = error
  return failure
}

function bindBackendMethod(value: object, name: string): (...args: any[]) => any {
  let owner: object | null = value
  while (owner !== null) {
    const descriptor = Object.getOwnPropertyDescriptor(owner, name)
    if (descriptor !== undefined) {
      if (!('value' in descriptor) || typeof descriptor.value !== 'function') {
        throw new TypeError(`backend session ${name} must be a data method`)
      }
      return descriptor.value.bind(value)
    }
    owner = Object.getPrototypeOf(owner)
  }
  throw new TypeError(`backend session is missing ${name}`)
}

function bindBackendSession<TPayloadFamily extends object>(
  value: object,
  close: SemanticBackendSession<TPayloadFamily>['close'],
): SemanticBackendSession<TPayloadFamily> {
  const evaluate = bindBackendMethod(value, 'evaluate') as SemanticBackendSession<TPayloadFamily>['evaluate']
  const validatePayload = bindBackendMethod(value, 'validatePayload') as SemanticBackendSession<TPayloadFamily>['validatePayload']
  const releasePayload = bindBackendMethod(value, 'releasePayload') as SemanticBackendSession<TPayloadFamily>['releasePayload']
  return Object.freeze({
    evaluate,
    validatePayload,
    releasePayload,
    close,
  })
}

function snapshotCloseRecord(
  value: unknown,
  expected: 'closed' | 'committed',
): (() => Promise<void>) | null {
  const invalid = () => new SemanticProgramExecutionError(
    'E_SEMANTIC_BACKEND_CLOSE',
    null,
    `Backend close did not return the required ${expected} ownership result`,
  )
  const record = snapshotDataRecord(value, invalid)
  if (expected === 'closed') {
    requireExactKeys(record, ['tag'], invalid)
    if (record.get('tag') !== 'closed') throw invalid()
    return null
  }
  requireExactKeys(record, ['tag', 'resultLease'], invalid)
  if (record.get('tag') !== 'committed') throw invalid()
  const resultLease = record.get('resultLease')
  const leaseRecord = snapshotDataRecord(resultLease, invalid)
  requireExactKeys(leaseRecord, ['dispose'], invalid)
  const dispose = leaseRecord.get('dispose')
  if (typeof dispose !== 'function') throw invalid()
  return () => Promise.resolve().then(() => dispose.call(resultLease))
}

function captureCommittedDisposer(value: unknown): (() => Promise<void>) | null {
  if (value === null || typeof value !== 'object') return null
  try {
    const descriptor = Object.getOwnPropertyDescriptor(value, 'resultLease')
    if (descriptor === undefined || !('value' in descriptor)
      || descriptor.value === null || typeof descriptor.value !== 'object') return null
    const dispose = bindBackendMethod(descriptor.value, 'dispose')
    return () => Promise.resolve().then(dispose)
  } catch {
    return null
  }
}

async function closeFailedSession<TPayloadFamily extends object>(
  session: SemanticBackendSession<TPayloadFamily>,
  controller: AbortController,
  primary: SemanticProgramExecutionError,
): Promise<never> {
  if (!controller.signal.aborted) controller.abort(primary)
  const outcome: SemanticBackendCloseOutcome = primary.code === 'E_SEMANTIC_ABORTED'
    || primary.code === 'E_SEMANTIC_DEADLINE'
    ? Object.freeze({ tag: 'abort', code: primary.code, node: primary.node })
    : Object.freeze({ tag: 'failure', code: primary.code, node: primary.node })
  try {
    const closed = await Promise.resolve().then(() => session.close(outcome))
    snapshotCloseRecord(closed, 'closed')
  } catch (cleanupError) {
    primary.cleanupError = cleanupError
  }
  throw primary
}

/** Executes only an atomically exact-source-lowered artifact, never a structural SPE1 alone. */
export async function executeSemanticProgram<TPayloadFamily extends object>(
  input: SemanticLoweringSuccess,
  backend: SemanticProgramBackend<TPayloadFamily>,
  control: SemanticExecutionControl = {},
): Promise<SemanticExecutionResult<TPayloadFamily>> {
  const trusted = requireTrustedSemanticLowering(input)
  const program = trusted.program
  const attestation = trusted.attestation
  const executionController = new AbortController()
  const nodes = program.core.nodes
  const maxNodes = control.maxNodes ?? nodes.length
  if (!Number.isSafeInteger(maxNodes) || maxNodes < 0 || nodes.length > maxNodes) {
    throw new SemanticProgramExecutionError('E_SEMANTIC_BUDGET', null, `Semantic program requires ${nodes.length} nodes but the effective limit is ${String(maxNodes)}`)
  }
  checkControl(control, null)

  let publishedSession: unknown
  try {
    publishedSession = backend.begin(Object.freeze({
      programHash: attestation.programHash,
      languageContract: program.core.language.contract,
      limits: Object.freeze({ maxNodes }),
      signal: executionController.signal,
    }))
  } catch (error) {
    const failure = new SemanticProgramExecutionError(
      'E_SEMANTIC_BACKEND_BEGIN',
      null,
      'Semantic backend could not begin an atomic execution session',
    )
    failure.backendCause = error
    throw failure
  }

  if (publishedSession === null || typeof publishedSession !== 'object') {
    const failure = new SemanticProgramExecutionError(
      'E_SEMANTIC_BACKEND_BEGIN', null, 'Semantic backend begin did not publish a session',
    )
    failure.backendCause = new TypeError('begin() did not return a session object')
    throw failure
  }
  let close: SemanticBackendSession<TPayloadFamily>['close']
  try {
    close = bindBackendMethod(publishedSession, 'close') as SemanticBackendSession<TPayloadFamily>['close']
  } catch (error) {
    const failure = new SemanticProgramExecutionError(
      'E_SEMANTIC_BACKEND_BEGIN', null, 'Published backend session has no callable data close method',
    )
    failure.backendCause = error
    throw failure
  }
  let session: SemanticBackendSession<TPayloadFamily>
  try {
    session = bindBackendSession<TPayloadFamily>(publishedSession, close)
  } catch (error) {
    const primary = new SemanticProgramExecutionError(
      'E_SEMANTIC_BACKEND_FAILURE', null, 'Published backend session has an invalid lifecycle shape',
    )
    primary.backendCause = error
    return closeFailedSession(
      Object.freeze({ close }) as SemanticBackendSession<TPayloadFamily>,
      executionController,
      primary,
    )
  }

  const values: SemanticRuntimeValue<TPayloadFamily>[] = new Array(nodes.length)
  const leasesByNode: Array<SemanticBackendPayloadLease | undefined> = []
  const seenLeases = new Set<SemanticBackendPayloadLease>()
  const producerByNode: Array<SemanticBackendContext['producer']> = Array(nodes.length).fill(null)
  for (const occurrence of program.core.occurrences) {
    if (occurrence.node === null || occurrence.outputOrdinal === null
      || producerByNode[occurrence.node] !== null) continue
    const operation = program.core.operations[occurrence.operation]
    if (!semanticNodeProducerOperationNames(nodes[occurrence.node]).includes(operation.name)) continue
    producerByNode[occurrence.node] = Object.freeze({
      occurrenceIndex: occurrence.id,
      operationIndex: operation.id,
      operationName: operation.name,
    })
  }
  let completedNodes = 0
  let activeNode: number | null = null
  try {
    for (const nodeIndex of program.core.execution.evaluationOrder) {
      const node = nodes[nodeIndex]
      activeNode = node.id
      checkControl(control, node.id)
      const authoredInputNodeIndices = semanticNodeInputs(node)
      const authoredInputs = Object.freeze(authoredInputNodeIndices.map(reference => values[reference]))
      const reduction = reduceEmptyInputs(node, authoredInputs)
      let value = reduction.result
      if (value === undefined) {
        const returned = await awaitBackendWithControl(() => session.evaluate(node, reduction.inputs, Object.freeze({
          nodeIndex: node.id,
          carrierKey: semanticCarrierKey(node.valueType),
          inputNodeIndices: Object.freeze(reduction.positions.map(position => authoredInputNodeIndices[position])),
          producer: producerByNode[node.id],
          programHash: attestation.programHash,
          languageContract: program.core.language.contract,
          signal: executionController.signal,
        })), control, executionController, node.id)
        checkControl(control, node.id)
        value = snapshotRuntimeValue(
          returned,
          node.valueType,
          node.id,
          program.core.language.contract === 'legacy/current',
          session,
          lease => {
          if (seenLeases.has(lease)) {
            throw new SemanticProgramExecutionError(
              'E_SEMANTIC_BACKEND_TYPE',
              node.id,
              'Backend reused a payload lease; every non-empty return requires a unique lease',
            )
          }
          seenLeases.add(lease)
          leasesByNode[node.id] = lease
          },
        )
        checkControl(control, node.id)
        if (value.tag === 'empty' && nodeMustReturnValueAfterReduction(node)) {
          throw new SemanticProgramExecutionError('E_SEMANTIC_BACKEND_TYPE', node.id, 'Backend returned empty where the shared semantic empty algebra requires a value')
        }
      }
      values[node.id] = value
      completedNodes++
      try {
        control.onNode?.(completedNodes, nodes.length)
      } catch {
        throw new SemanticProgramExecutionError('E_SEMANTIC_CALLBACK', node.id, 'Semantic execution progress callback failed')
      }
      checkControl(control, node.id)
      activeNode = null
    }
    checkControl(control, null)
  } catch (error) {
    return closeFailedSession(session, executionController, executionFailure(error, activeNode))
  }

  const terminal = program.core.execution.terminal
  if (terminal !== null) {
    const failure = new SemanticProgramExecutionError(
      'E_SEMANTIC_LANGUAGE_TERMINAL',
      null,
      'Semantic kernel prefix completed before the deterministic language terminal',
    )
    failure.terminalDiagnostic = terminal.diagnosticTemplate
    return closeFailedSession(session, executionController, failure)
  }

  let outputs: readonly SemanticExecutedOutput<TPayloadFamily>[]
  let retained: readonly SemanticBackendPayloadLease[]
  try {
    const resultItems = semanticResultItems(program.core.result)
    outputs = Object.freeze(resultItems.map(output => Object.freeze({
      value: values[output.node],
      occurrence: program.core.occurrences[output.identityOccurrence],
      color: output.color,
    })))
    retained = Object.freeze([...new Set(resultItems
      .map(output => leasesByNode[output.node])
      .filter((lease): lease is SemanticBackendPayloadLease => lease !== undefined))])
  } catch (error) {
    return closeFailedSession(session, executionController, executionFailure(error, null))
  }

  let committed: SemanticBackendCloseResult
  try {
    committed = await Promise.resolve().then(() => session.close(Object.freeze({
      tag: 'commit' as const,
      retained,
    })))
  } catch (error) {
    const failure = new SemanticProgramExecutionError(
      'E_SEMANTIC_BACKEND_CLOSE',
      null,
      'Semantic backend failed its atomic commit-close boundary',
    )
    failure.backendCause = error
    throw failure
  }

  const emergencyDispose = captureCommittedDisposer(committed)
  let disposeBackendResult: (() => Promise<void>) | null = null
  try {
    disposeBackendResult = snapshotCloseRecord(committed, 'committed')
    if (disposeBackendResult === null) throw new SemanticProgramExecutionError(
      'E_SEMANTIC_BACKEND_CLOSE', null, 'Backend commit omitted its result lease',
    )
  } catch (error) {
    const primary = error instanceof SemanticProgramExecutionError
      ? error
      : new SemanticProgramExecutionError(
        'E_SEMANTIC_BACKEND_CLOSE', null, 'Backend returned an invalid commit ownership record',
      )
    if (emergencyDispose !== null) {
      try {
        await emergencyDispose()
      } catch (cleanupError) {
        primary.cleanupError = cleanupError
      }
    }
    throw primary
  }

  try {
    let disposePromise: Promise<void> | undefined
    let disposed = false
    const dispose = (): Promise<void> => {
      if (disposePromise === undefined) {
        disposed = true
        disposePromise = Promise.resolve().then(disposeBackendResult)
      }
      return disposePromise
    }
    return Object.freeze({
      program,
      attestation,
      outputs,
      get disposed() { return disposed },
      dispose,
      [Symbol.asyncDispose]: dispose,
    })
  } catch (error) {
    try {
      await disposeBackendResult()
    } catch (cleanupError) {
      const failure = executionFailure(error, null)
      failure.cleanupError = cleanupError
      throw failure
    }
    throw executionFailure(error, null)
  }
}
