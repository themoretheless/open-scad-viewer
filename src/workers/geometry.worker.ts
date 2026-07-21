import { parseOpenSCAD, OpenSCADParseError } from '../services/openscadParser'
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

  try {
    // parseOpenSCAD/Manifold is synchronous after WASM initialization. A
    // cancel message cannot interrupt that section of the worker event loop;
    // BuildCoordinator therefore uses worker replacement for hard preemption.
    const result = await parseOpenSCAD(request.source, { quality: request.quality })
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
  // Cancellation is only acknowledged here. In particular, do not publish a
  // terminal event while parseOpenSCAD may still be initializing or executing
  // synchronous Manifold work. runBuild publishes cancellation after the next
  // real checkpoint; if it cannot reach one, BuildCoordinator's grace timer
  // replaces this worker and establishes the hard cancellation boundary.
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
