import { parentPort } from 'node:worker_threads'
import { meshTransferables } from '../core/mesh'
import { evaluateOpenSCADViaManifoldPlanForQualification } from '../services/manifoldPlanEvaluator'
import {
  MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
  isMcpManifoldPlanQualificationCancel,
  isMcpManifoldPlanQualificationNodeTerminal,
  isMcpManifoldPlanQualificationRequest,
  isMcpManifoldPlanQualificationStarted,
  serializeMcpManifoldPlanQualificationError,
  type McpManifoldPlanQualificationRequest,
  type McpManifoldPlanQualificationTerminal,
} from '../services/manifoldPlanQualificationProtocol'

if (parentPort === null) throw new Error('Manifold qualification worker requires a parent port')

let active: McpManifoldPlanQualificationRequest | null = null
let cancelled = false
let terminalPosted = false

function terminalEnvelope(request: McpManifoldPlanQualificationRequest) {
  return {
    protocolVersion: MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
    workerEpoch: request.workerEpoch,
    jobId: request.jobId,
    sourceSha256: request.sourceSha256,
    identity: MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  } as const
}

function postTerminal(terminal: McpManifoldPlanQualificationTerminal): void {
  if (terminalPosted) return
  const publishable = active !== null && isMcpManifoldPlanQualificationNodeTerminal(terminal, active)
    ? terminal
    : active === null
      ? null
      : {
          ...terminalEnvelope(active),
          status: 'failed' as const,
          error: {
            name: 'ManifoldPlanQualificationProtocolError',
            message: 'Manifold plan result cannot be published by the qualification child',
            code: 'QUALIFICATION_RESULT_UNPUBLISHABLE',
            line: null,
            column: null,
            start: null,
            end: null,
          },
        }
  if (publishable === null) {
    parentPort!.close()
    return
  }
  if (publishable.status === 'succeeded') {
    parentPort!.postMessage(publishable, meshTransferables(publishable.result.meshes))
  } else {
    parentPort!.postMessage(publishable)
  }
  // A failed structured clone must remain recoverable by evaluate()'s catch
  // path. Claim the terminal only after the parent port accepted the message.
  terminalPosted = true
  parentPort!.close()
}

function postStarted(request: McpManifoldPlanQualificationRequest): void {
  const started = {
    ...terminalEnvelope(request),
    status: 'started' as const,
  }
  if (!isMcpManifoldPlanQualificationStarted(started, request)) {
    throw new TypeError('Invalid Manifold qualification started handshake')
  }
  parentPort!.postMessage(started)
}

async function evaluate(request: McpManifoldPlanQualificationRequest): Promise<void> {
  try {
    const result = await evaluateOpenSCADViaManifoldPlanForQualification(request.source, {
      quality: request.quality,
      shouldAbort: () => cancelled,
    })
    postTerminal({
      ...terminalEnvelope(request),
      status: 'succeeded',
      result,
    })
  } catch (error) {
    postTerminal({
      ...terminalEnvelope(request),
      status: 'failed',
      error: serializeMcpManifoldPlanQualificationError(error),
    })
  }
}

parentPort.on('message', value => {
  if (active === null) {
    if (!isMcpManifoldPlanQualificationRequest(value)) return
    active = value
    postStarted(value)
    void evaluate(value)
    return
  }
  if (isMcpManifoldPlanQualificationCancel(value, active)) cancelled = true
})
