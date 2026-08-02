import { Worker } from 'node:worker_threads'
import { serveStdio, StdioServerTransport } from '@modelcontextprotocol/server/stdio'
import type { GeometryQuality } from '../../src/core/build'
import { BoundedTransport, MAX_MCP_SUBSCRIPTIONS } from '../../src/mcp/boundedTransport'
import { createOpenScadMcpServer } from '../../src/mcp/createServer'
import { DuckDbModelStore } from '../../src/mcp/duckdbModelStore'
import { HeadlessGeometryService } from '../../src/mcp/geometryService'
import {
  McpManifoldPlanQualificationSupervisor,
  McpManifoldPlanSupervisorError,
} from '../../src/mcp/manifoldPlanQualificationSupervisor'

const eventPrefix = 'QUALIFICATION_EVENT '

function report(event: string, details: Record<string, unknown> = {}): void {
  process.stderr.write(`${eventPrefix}${JSON.stringify({ event, ...details })}\n`)
}

function delay(milliseconds: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, milliseconds))
}

class StdioQualificationShadowGeometry extends HeadlessGeometryService {
  private readonly entered = new Int32Array(new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT))
  private readonly supervisor: McpManifoldPlanQualificationSupervisor
  private shadowStarted = false
  private shadowSettlements = 0
  private shadowTask: Promise<void> | null = null
  private entryTask: Promise<void> | null = null

  constructor() {
    super()
    this.supervisor = new McpManifoldPlanQualificationSupervisor({
      deadlineMs: 2_500,
      cancellationGraceMs: 10,
      workerFactory: workerEpoch => {
        const worker = new Worker(
          new URL('./mcp-shadow-noncooperative.worker.mjs', import.meta.url),
          {
            workerData: { entered: this.entered.buffer, workerEpoch },
            stdout: true,
            stderr: true,
          },
        )
        let capturedBytes = 0
        worker.stdout.setEncoding('utf8')
        worker.stdout.on('data', chunk => {
          capturedBytes += Buffer.byteLength(chunk)
          if (capturedBytes <= 4_096 && chunk.includes('qualification-worker-private-stdout')) {
            report('shadow-stdout-captured', { workerEpoch })
          }
          if (capturedBytes > 4_096) void worker.terminate()
        })
        let capturedErrorBytes = 0
        worker.stderr.on('data', chunk => {
          capturedErrorBytes += Buffer.byteLength(chunk)
          if (capturedErrorBytes > 4_096) void worker.terminate()
        })
        return worker
      },
    })
  }

  override async analyze(
    source: string,
    quality: GeometryQuality = 'full',
    signal?: AbortSignal,
  ) {
    // The direct production-v5 result is fixed first. The injected shadow is
    // deliberately fire-and-observe and has no reference to the ModelStore.
    const primary = await super.analyze(source, quality, signal)
    if (!this.shadowStarted) this.startQualificationShadow(source, quality)
    return primary
  }

  private startQualificationShadow(source: string, quality: GeometryQuality): void {
    this.shadowStarted = true
    const evaluation = this.supervisor.evaluate(source, quality)
    this.entryTask = (async () => {
      while (Atomics.load(this.entered, 0) !== 1 && this.shadowSettlements === 0) await delay(2)
      if (Atomics.load(this.entered, 0) === 1) {
        report('shadow-entered-noncooperative-loop', {
          workerEpoch: this.supervisor.snapshot().activeWorkerEpoch,
        })
      } else {
        report('shadow-did-not-enter-loop')
      }
    })()
    this.shadowTask = evaluation.then(
      () => {
        this.shadowSettlements++
        report('shadow-settled', {
          outcome: 'unexpected-success',
          settlements: this.shadowSettlements,
          snapshot: this.supervisor.snapshot(),
        })
      },
      error => {
        this.shadowSettlements++
        report('shadow-settled', {
          outcome: error instanceof McpManifoldPlanSupervisorError ? error.code : 'unexpected-error',
          settlements: this.shadowSettlements,
          snapshot: this.supervisor.snapshot(),
        })
      },
    )
  }

  async closeQualificationShadow(): Promise<void> {
    await Promise.all([this.entryTask, this.shadowTask].filter(
      (task): task is Promise<void> => task !== null,
    ))
  }
}

const store = await DuckDbModelStore.open(':memory:')
const geometry = new StdioQualificationShadowGeometry()
const transport = new BoundedTransport(new StdioServerTransport(), {
  maxInFlightRequests: 8,
  maxPendingOutboundMessages: 24,
})
const handle = serveStdio(() => createOpenScadMcpServer({
  store,
  geometry,
  onRequestSettled: requestId => transport.settleRequest(requestId),
  onRequestCancelled: requestId => transport.cancelRequest(requestId),
}), {
  transport,
  maxSubscriptions: MAX_MCP_SUBSCRIPTIONS,
})

let closePromise: Promise<void> | null = null
function close(): Promise<void> {
  closePromise ??= (async () => {
    await geometry.closeQualificationShadow()
    const outcomes = await Promise.allSettled([handle.close(), store.close()])
    const failures = outcomes
      .filter((outcome): outcome is PromiseRejectedResult => outcome.status === 'rejected')
      .map(outcome => outcome.reason)
    if (failures.length) throw new AggregateError(failures, 'Qualification stdio shutdown failed')
  })()
  return closePromise
}

process.stdin.once('end', () => {
  void close().catch(error => {
    report('server-close-failed', { message: error instanceof Error ? error.message : String(error) })
    process.exitCode = 1
  })
})
process.once('SIGTERM', () => void close())
process.once('SIGINT', () => void close())
report('server-ready')
