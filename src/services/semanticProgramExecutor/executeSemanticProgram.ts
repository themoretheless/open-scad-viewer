import { SemanticProgramExecutionError } from '../semanticProgramExecutor'
import type { SemanticBackendCloseResult, SemanticBackendContext, SemanticBackendPayloadLease, SemanticBackendSession, SemanticExecutedOutput, SemanticExecutionControl, SemanticExecutionResult, SemanticProgramBackend, SemanticRuntimeValue } from '../semanticProgramExecutor'
import { requireTrustedSemanticLowering } from '../semanticProgramLowerer'
import type { SemanticLoweringSuccess } from '../semanticProgramLowerer'
import { awaitBackendWithControl, bindBackendMethod, bindBackendSession, captureCommittedDisposer, checkSemanticExecutionControl, closeFailedSession, executionFailure, nodeMustReturnValueAfterReduction, reduceEmptyInputs, semanticCarrierKey, snapshotCloseRecord, snapshotRuntimeValue } from './checkSemanticExecutionControl'
import { semanticNodeInputs, semanticNodeProducerOperationNames, semanticResultItems } from '../../core/semanticProgram'

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
  checkSemanticExecutionControl(control, null)

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
      checkSemanticExecutionControl(control, node.id)
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
        checkSemanticExecutionControl(control, node.id)
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
        checkSemanticExecutionControl(control, node.id)
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
      checkSemanticExecutionControl(control, node.id)
      activeNode = null
    }
    checkSemanticExecutionControl(control, null)
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
