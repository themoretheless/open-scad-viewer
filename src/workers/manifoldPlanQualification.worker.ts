import { meshTransferables } from '../core/mesh'
import {
  MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
  isMcpManifoldPlanQualificationCancel,
  isMcpManifoldPlanQualificationRequest,
  isMcpManifoldPlanQualificationStarted,
  isMcpManifoldPlanQualificationTerminal,
  serializeMcpManifoldPlanQualificationError,
  type McpManifoldPlanQualificationRequest,
  type McpManifoldPlanQualificationTerminal,
} from '../services/manifoldPlanQualificationProtocol'
import { evaluateOpenSCADViaManifoldPlanForQualification } from '../services/manifoldPlanEvaluator'

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
  if (terminalPosted || active === null) return
  const publishable = isMcpManifoldPlanQualificationTerminal(terminal, active)
    ? terminal
    : {
        ...terminalEnvelope(active),
        status: 'failed' as const,
        error: {
          name: 'ManifoldPlanQualificationProtocolError',
          message: 'Manifold plan result cannot be published by the qualification Worker',
          code: 'QUALIFICATION_RESULT_UNPUBLISHABLE',
          line: null,
          column: null,
          start: null,
          end: null,
        },
      }
  try {
    if (publishable.status === 'succeeded') {
      self.postMessage(publishable, { transfer: meshTransferables(publishable.result.meshes) })
    } else {
      self.postMessage(publishable)
    }
    terminalPosted = true
  } catch (error) {
    if (publishable.status !== 'succeeded') throw error
    self.postMessage({
      ...terminalEnvelope(active),
      status: 'failed',
      error: {
        name: 'ManifoldPlanQualificationProtocolError',
        message: 'Manifold plan result could not cross the qualification Worker boundary',
        code: 'QUALIFICATION_RESULT_CLONE_FAILED',
        line: null,
        column: null,
        start: null,
        end: null,
      },
    })
    terminalPosted = true
  }
  self.close()
}

function postStarted(request: McpManifoldPlanQualificationRequest): void {
  const started = { ...terminalEnvelope(request), status: 'started' as const }
  if (!isMcpManifoldPlanQualificationStarted(started, request)) {
    throw new Error('Invalid Manifold plan qualification startup acknowledgement')
  }
  self.postMessage(started)
}

async function evaluate(request: McpManifoldPlanQualificationRequest): Promise<void> {
  try {
    const result = await evaluateOpenSCADViaManifoldPlanForQualification(request.source, {
      quality: request.quality,
      shouldAbort: () => cancelled,
    })
    postTerminal({ ...terminalEnvelope(request), status: 'succeeded', result })
  } catch (error) {
    postTerminal({
      ...terminalEnvelope(request),
      status: 'failed',
      error: serializeMcpManifoldPlanQualificationError(error),
    })
  }
}

self.addEventListener('message', (event: MessageEvent<unknown>) => {
  if (active === null) {
    if (!isMcpManifoldPlanQualificationRequest(event.data)) return
    active = event.data
    postStarted(event.data)
    void evaluate(event.data)
    return
  }
  if (isMcpManifoldPlanQualificationCancel(event.data, active)) cancelled = true
})
