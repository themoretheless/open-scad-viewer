import { AbortedError, getWasm, parseOpenSCAD, OpenSCADParseError } from '../services/openscadParser'
import { meshTransferables } from '../core/mesh'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  isGeometryWorkerRequest,
  type GeometryBuildRequest,
  type GeometryBuildTerminal,
  type GeometryCancelReason,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from '../services/geometryWorkerProtocol'

interface ActiveJob {
  request: GeometryBuildRequest
  startedAt: number
  cancelled: GeometryCancelReason | null
}

const activeJobs = new Map<number, ActiveJob>()
const latestByQuality = new Map<GeometryBuildRequest['quality'], GeometryBuildRequest>()
let latestDocumentRevision = -1

// Eagerly warm the Manifold WASM at startup so the first build does not pay
// the download+compile cost. A warm-up failure is not fatal: getWasm() does
// not cache rejections, so the first real build simply retries the load.
void getWasm().catch(() => undefined)

function postEvent(event: GeometryWorkerEvent, transfer: Transferable[] = []) {
  self.postMessage(event, { transfer })
}

function jobEvent<T extends Omit<GeometryWorkerEvent, 'protocolVersion' | 'documentRevision' | 'jobId' | 'quality'>>(
  request: GeometryBuildRequest,
  event: T,
): GeometryWorkerEvent {
  return {
    protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
    documentRevision: request.documentRevision,
    jobId: request.jobId,
    quality: request.quality,
    ...event,
  } as GeometryWorkerEvent
}

function elapsed(job: ActiveJob) {
  return performance.now() - job.startedAt
}

/** Minimum spacing between mid-parse liveness heartbeat progress events. */
const HEARTBEAT_THROTTLE_MS = 1000

function staleReplacement(request: GeometryBuildRequest) {
  if (request.documentRevision !== latestDocumentRevision) {
    const latest = [...latestByQuality.values()].reduce<GeometryBuildRequest | undefined>((best, candidate) => (
      !best || candidate.jobId > best.jobId ? candidate : best
    ), undefined)
    return latest ? { documentRevision: latest.documentRevision, jobId: latest.jobId } : undefined
  }
  const latest = latestByQuality.get(request.quality)
  return latest && latest.jobId !== request.jobId
    ? { documentRevision: latest.documentRevision, jobId: latest.jobId }
    : undefined
}

function terminalState(job: ActiveJob, phase: GeometryBuildTerminal['phase']): GeometryBuildTerminal | null {
  const { request } = job
  if (job.cancelled) {
    return jobEvent(request, {
      status: 'cancelled',
      phase,
      reason: job.cancelled,
      durationMs: elapsed(job),
    }) as GeometryBuildTerminal
  }
  const supersededBy = staleReplacement(request)
  if (supersededBy) {
    return jobEvent(request, {
      status: 'stale',
      phase,
      supersededBy,
      durationMs: elapsed(job),
    }) as GeometryBuildTerminal
  }
  return null
}

async function runBuild(request: GeometryBuildRequest) {
  const job: ActiveJob = {
    request,
    startedAt: performance.now(),
    cancelled: null,
  }
  activeJobs.set(request.jobId, job)
  postEvent(jobEvent(request, { status: 'accepted', phase: 'queued' }))

  const alreadyStale = terminalState(job, 'queued')
  if (alreadyStale) {
    postEvent(alreadyStale)
    activeJobs.delete(request.jobId)
    return
  }

  postEvent(jobEvent(request, { status: 'started', phase: 'initializing' }))
  postEvent(jobEvent(request, { status: 'progress', phase: 'compiling', progress: null }))

  let lastHeartbeat = performance.now()
  try {
    // The parser's top-level statement loop yields to the event loop
    // periodically, so queued cancel messages are delivered mid-parse and
    // shouldAbort stops a superseded/cancelled evaluation cooperatively — the
    // warm worker (and its cached WASM) survives. A single wedged statement
    // still cannot yield; BuildCoordinator's grace timer replaces the worker
    // in that case and remains the hard cancellation boundary.
    const result = await parseOpenSCAD(request.source, {
      quality: request.quality,
      shouldAbort: () => job.cancelled !== null || staleReplacement(request) !== undefined,
      onYield: () => {
        // Throttled liveness heartbeat: the build is alive (just heavy). The
        // main thread can distinguish honest long builds from a wedged worker
        // instead of killing them at a fixed timeout.
        const now = performance.now()
        if (now - lastHeartbeat >= HEARTBEAT_THROTTLE_MS) {
          lastHeartbeat = now
          postEvent(jobEvent(request, { status: 'progress', phase: 'compiling', progress: null }))
        }
      },
    })
    const terminal = terminalState(job, 'compiling')
    if (terminal) {
      postEvent(terminal)
      return
    }

    postEvent(jobEvent(request, { status: 'progress', phase: 'serializing', progress: 0.95 }))
    const response = jobEvent(request, {
      status: 'succeeded',
      phase: 'complete',
      ...result,
      durationMs: elapsed(job),
    })
    postEvent(response, meshTransferables(result.meshes))
  } catch (error) {
    const terminal = terminalState(job, 'compiling')
    if (terminal) {
      postEvent(terminal)
      return
    }
    if (error instanceof AbortedError) {
      // shouldAbort only returns true when the job was cancelled or
      // superseded, so terminalState above covers this in practice. Guard the
      // race anyway: an aborted evaluation must never surface as a build error.
      postEvent(jobEvent(request, {
        status: 'cancelled',
        phase: 'compiling',
        reason: 'superseded',
        durationMs: elapsed(job),
      }))
      return
    }
    postEvent(jobEvent(request, {
      status: 'failed',
      phase: 'compiling',
      error: {
        name: error instanceof Error ? error.name : 'Error',
        message: error instanceof Error ? error.message : String(error),
        line: error instanceof OpenSCADParseError ? error.line : undefined,
        column: error instanceof OpenSCADParseError ? error.column : undefined,
      },
      durationMs: elapsed(job),
    }))
  } finally {
    activeJobs.delete(request.jobId)
  }
}

function acceptBuild(request: GeometryBuildRequest) {
  if (request.documentRevision > latestDocumentRevision) {
    latestDocumentRevision = request.documentRevision
    latestByQuality.clear()
  }
  if (request.documentRevision === latestDocumentRevision) {
    const previous = latestByQuality.get(request.quality)
    if (!previous || request.jobId > previous.jobId) latestByQuality.set(request.quality, request)
  }
  void runBuild(request)
}

function cancelBuild(request: Extract<GeometryWorkerRequest, { type: 'cancel' }>) {
  const job = activeJobs.get(request.jobId)
  if (!job || job.request.documentRevision !== request.documentRevision || job.cancelled) return
  // Cancellation is only acknowledged here. Do not publish a terminal event
  // directly: the running evaluation observes this flag through shouldAbort at
  // its next cooperative yield (or checkpoint) and runBuild publishes the
  // cancellation there. If the evaluation is wedged inside one statement and
  // never yields, BuildCoordinator's grace timer replaces this worker and
  // establishes the hard cancellation boundary.
  job.cancelled = request.reason
}

self.addEventListener('message', (event: MessageEvent<unknown>) => {
  // Validation happens before any lifecycle state changes, so a malformed ID
  // cannot poison later work. Every accepted build reaches an explicit
  // succeeded/failed/stale/cancelled terminal unless the worker is replaced.
  if (isGeometryWorkerRequest(event.data)) {
    if (event.data.type === 'build') acceptBuild(event.data)
    else cancelBuild(event.data)
  }
})
