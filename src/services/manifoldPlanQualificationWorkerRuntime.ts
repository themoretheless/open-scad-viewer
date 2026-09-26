import { meshTransferables } from '../core/mesh'
import { evaluateOpenSCADViaManifoldPlanForQualification } from './manifoldPlanEvaluator'
import {
  MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
  isMcpManifoldPlanQualificationCancel,
  isMcpManifoldPlanQualificationRequest,
  isMcpManifoldPlanQualificationStarted,
  serializeMcpManifoldPlanQualificationError,
  type McpManifoldPlanQualificationRequest,
  type McpManifoldPlanQualificationStarted,
  type McpManifoldPlanQualificationTerminal,
} from './manifoldPlanQualificationProtocol'

/**
 * Transport-specific behaviour of a qualification lane. The shared runtime
 * below owns the protocol state machine; the lane only adapts the wire
 * (browser `self` vs Node `parentPort`) and the lane-specific publish
 * guards/messages.
 */
export interface ManifoldPlanQualificationWorkerLane {
  post(
    message: McpManifoldPlanQualificationStarted | McpManifoldPlanQualificationTerminal,
    transfer: ArrayBuffer[],
  ): void
  close(): void
  /** Lane-specific terminal guard (the Node lane adds its stricter IPC budget). */
  isTerminalPublishable(
    terminal: McpManifoldPlanQualificationTerminal,
    request: McpManifoldPlanQualificationRequest,
  ): boolean
  unpublishableErrorMessage: string
  /**
   * When set, a failed structured clone of a success result is recovered
   * in-band with a QUALIFICATION_RESULT_CLONE_FAILED failure. When null, the
   * post error propagates so evaluate()'s catch path can retry with a
   * failure terminal (the Node lane relies on this).
   */
  cloneFailureErrorMessage: string | null
  invalidStartedHandshakeError(): Error
}

export function manifoldPlanQualificationWorkerRuntime(
  lane: ManifoldPlanQualificationWorkerLane,
): (value: unknown) => void {
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

  function protocolError(message: string, code: string) {
    return {
      name: 'ManifoldPlanQualificationProtocolError',
      message,
      code,
      line: null,
      column: null,
      start: null,
      end: null,
    } as const
  }

  function postTerminal(terminal: McpManifoldPlanQualificationTerminal): void {
    if (terminalPosted) return
    const publishable: McpManifoldPlanQualificationTerminal | null =
      active !== null && lane.isTerminalPublishable(terminal, active)
        ? terminal
        : active === null
          ? null
          : {
              ...terminalEnvelope(active),
              status: 'failed' as const,
              error: protocolError(lane.unpublishableErrorMessage, 'QUALIFICATION_RESULT_UNPUBLISHABLE'),
            }
    if (publishable === null) {
      lane.close()
      return
    }
    const transfer = publishable.status === 'succeeded'
      ? meshTransferables(publishable.result.meshes)
      : []
    if (lane.cloneFailureErrorMessage === null) {
      // A failed post must remain recoverable by evaluate()'s catch path.
      // Claim the terminal only after the port accepted the message.
      lane.post(publishable, transfer)
      terminalPosted = true
    } else {
      try {
        lane.post(publishable, transfer)
        terminalPosted = true
      } catch (error) {
        if (publishable.status !== 'succeeded') throw error
        lane.post({
          ...terminalEnvelope(active!),
          status: 'failed',
          error: protocolError(lane.cloneFailureErrorMessage, 'QUALIFICATION_RESULT_CLONE_FAILED'),
        }, [])
        terminalPosted = true
      }
    }
    lane.close()
  }

  function postStarted(request: McpManifoldPlanQualificationRequest): void {
    const started = { ...terminalEnvelope(request), status: 'started' as const }
    if (!isMcpManifoldPlanQualificationStarted(started, request)) {
      throw lane.invalidStartedHandshakeError()
    }
    lane.post(started, [])
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

  return (value: unknown) => {
    if (active === null) {
      if (!isMcpManifoldPlanQualificationRequest(value)) return
      active = value
      postStarted(value)
      void evaluate(value)
      return
    }
    if (isMcpManifoldPlanQualificationCancel(value, active)) cancelled = true
  }
}
