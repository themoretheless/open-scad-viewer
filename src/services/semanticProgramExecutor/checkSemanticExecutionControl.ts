import { SemanticProgramExecutionError } from '../semanticProgramExecutor'
import type { SemanticBackendCloseOutcome, SemanticBackendPayloadLease, SemanticBackendSession, SemanticCarrierKey, SemanticExecutionControl, SemanticRuntimeValue } from '../semanticProgramExecutor'
import type { SemanticNode, SemanticValueType } from '../../core/semanticProgram'

export function semanticCarrierKey(valueType: SemanticValueType): SemanticCarrierKey {
  return `${valueType.geometryKind}/${valueType.space}/${valueType.representation}`
}

export function checkSemanticExecutionControl(control: SemanticExecutionControl, node: number | null): void {
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

export async function awaitBackendWithControl<T>(
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

export function snapshotDataRecord(
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

export function requireExactKeys(
  record: ReadonlyMap<string, unknown>,
  expected: readonly string[],
  invalid: () => SemanticProgramExecutionError,
): void {
  if (record.size !== expected.length || expected.some(key => !record.has(key))) throw invalid()
}

export function snapshotValueType(
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

export function snapshotRuntimeValue<TPayloadFamily extends object>(
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

export function typedEmpty<TPayloadFamily extends object>(
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

export function reduceEmptyInputs<TPayloadFamily extends object>(
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

export function nodeMustReturnValueAfterReduction(node: SemanticNode): boolean {
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

export function executionFailure(error: unknown, node: number | null): SemanticProgramExecutionError {
  if (error instanceof SemanticProgramExecutionError) return error
  const failure = new SemanticProgramExecutionError(
    'E_SEMANTIC_BACKEND_FAILURE',
    node,
    'Semantic backend evaluation failed',
  )
  failure.backendCause = error
  return failure
}

export function bindBackendMethod(value: object, name: string): (...args: any[]) => any {
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

export function bindBackendSession<TPayloadFamily extends object>(
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

export function snapshotCloseRecord(
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

export function captureCommittedDisposer(value: unknown): (() => Promise<void>) | null {
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

export async function closeFailedSession<TPayloadFamily extends object>(
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
