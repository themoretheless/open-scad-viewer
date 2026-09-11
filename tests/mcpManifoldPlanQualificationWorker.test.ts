import { afterEach, describe, expect, it, vi } from 'vitest'
import { sha256Hex } from '../src/core/sha256'
import {
  MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
  type McpManifoldPlanQualificationRequest,
} from '../src/services/manifoldPlanQualificationProtocol'

const evaluateMock = vi.hoisted(() => vi.fn())
const port = vi.hoisted(() => {
  let messageListener: ((value: unknown) => void) | undefined
  const posted: unknown[] = []
  let closes = 0
  return {
    posted,
    get closes() { return closes },
    reset() {
      messageListener = undefined
      posted.length = 0
      closes = 0
    },
    dispatch(value: unknown) {
      messageListener?.(value)
    },
    parentPort: {
      on(type: string, listener: (value: unknown) => void) {
        if (type === 'message') messageListener = listener
      },
      postMessage(value: unknown) {
        if ((value as { status?: unknown })?.status === 'succeeded') {
          const error = new Error('fixture structured clone failed')
          error.name = 'DataCloneError'
          throw error
        }
        posted.push(value)
      },
      close() {
        closes++
      },
    },
  }
})

vi.mock('node:worker_threads', () => ({ parentPort: port.parentPort }))
vi.mock('../src/services/manifoldPlanEvaluator', () => ({
  evaluateOpenSCADViaManifoldPlanForQualification: evaluateMock,
}))

afterEach(() => {
  evaluateMock.mockReset()
  port.reset()
  vi.resetModules()
})

describe('MCP Manifold-plan qualification Node worker', () => {
  it('claims terminalPosted only after a successful post and recovers clone failure as one failed terminal', async () => {
    evaluateMock.mockResolvedValue({
      meshes: [],
      warnings: [],
      volume: 0,
      surfaceArea: 0,
      quality: 'full',
      reduced: false,
      timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
    })
    await import('../src/mcp/manifoldPlanQualification.worker')
    const source = 'cube(1);'
    const request: McpManifoldPlanQualificationRequest = {
      protocolVersion: MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
      type: 'evaluate',
      workerEpoch: 1,
      jobId: 1,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
      identity: MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
    }

    port.dispatch(request)

    await vi.waitFor(() => expect(port.posted.at(-1)).toMatchObject({
      status: 'failed',
      error: {
        name: 'DataCloneError',
        message: 'fixture structured clone failed',
      },
    }))
    expect(port.posted.map(value => (value as { status?: unknown }).status))
      .toEqual(['started', 'failed'])
    expect(port.closes).toBe(1)
  })
})
