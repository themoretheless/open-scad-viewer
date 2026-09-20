import {
  defaultGeometryBuildEngine,
  geometryExecutionForError,
  GeometryCapabilityUnavailableError,
  GeometryEngineUnavailableError,
  GeometryLanguageContractError,
} from '../services/geometryBuildEngine'
import { AbortedError, OpenSCADParseError } from '../services/openscadParser'
import { isExactSolidRequest } from '../services/solid/exactSolidProtocol'
import { meshTransferables } from '../core/mesh'
import {createSelectionSurfacePublisher} from '../services/selectionSurfacePublisher'
import {surfaceGroupsInKernel} from '../services/geometry/meshAnalysis'
import {warmGeometryKernel} from '../services/geometry/kernel'
import type { GeometryExecutionDescriptor } from '../core/geometryExecution'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  isGeometryWorkerEvent,
  isGeometryWorkerRequest,
  type GeometryBuildError,
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
  initialization: AbortController
}

const activeJobs = new Map<number, ActiveJob>()
const publishSelectionSurfaces = createSelectionSurfacePublisher(surfaceGroupsInKernel)
const latestByQuality = new Map<GeometryBuildRequest['quality'], GeometryBuildRequest>()
let latestDocumentRevision = -1
// BuildCoordinator emits monotonically increasing ids over a FIFO MessagePort.
// A bounded high-water tombstone prevents both active and post-terminal replay
// without retaining an unbounded set of completed jobs for the Worker lifetime.
let highestAcceptedJobId = -1
let exactJobStarted = false

const UNPUBLISHABLE_RESULT_ERROR = Object.freeze({
  name: 'GeometryWorkerProtocolError',
  code: 'WORKER_RESULT_UNPUBLISHABLE',
  message: `Geometry result cannot be published under protocol v${GEOMETRY_WORKER_PROTOCOL_VERSION}`,
} satisfies GeometryBuildError)

function postEvent(event: GeometryWorkerEvent, transfer: Transferable[] = []) {
  self.postMessage(event, { transfer })
}

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, Extract<keyof T, K>> : never
type GeometryJobEventPayload = DistributiveOmit<
  GeometryWorkerEvent,
  'protocolVersion' | 'documentRevision' | 'jobId' | 'quality' | 'sourceSha256'
>

function jobEvent<T extends GeometryJobEventPayload>(
  request: GeometryBuildRequest,
  event: T,
): GeometryWorkerEvent {
  return {
    protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
    documentRevision: request.documentRevision,
    jobId: request.jobId,
    quality: request.quality,
    sourceSha256: request.sourceSha256,
    ...event,
  } as GeometryWorkerEvent
}

function elapsed(job: ActiveJob) {
  return performance.now() - job.startedAt
}

function serializedBuildError(error: unknown): GeometryBuildError {
  const base = {
    name: error instanceof Error ? error.name : 'Error',
    message: error instanceof Error ? error.message : String(error),
  }
  if (error instanceof OpenSCADParseError) {
    return {
      ...base,
      ...(error.code === undefined ? {} : { code: error.code }),
      start: error.start,
      end: error.end,
      line: error.line,
      column: error.column,
    }
  }
  if (error instanceof GeometryEngineUnavailableError) return { ...base, code: 'ENGINE_UNAVAILABLE' }
  if (error instanceof GeometryCapabilityUnavailableError) return { ...base, code: 'CAPABILITY_UNAVAILABLE' }
  if (error instanceof GeometryLanguageContractError) {
    return {
      ...base,
      code: 'LANGUAGE_CONTRACT_UNSUPPORTED',
      ...(error.line === null ? {} : { line: error.line }),
    }
  }
  return base
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

function plannedExecution(request: GeometryBuildRequest) {
  try {
    return defaultGeometryBuildEngine.planSource(request.source, {
      quality: request.quality,
      purpose: request.quality,
    })
  } catch {
    return undefined
  }
}

function terminalState(
  job: ActiveJob,
  phase: GeometryBuildTerminal['phase'],
  execution = plannedExecution(job.request),
): GeometryBuildTerminal | null {
  const { request } = job
  if (job.cancelled) {
    return jobEvent(request, {
      status: 'cancelled',
      phase,
      ...(execution ? { execution } : {}),
      reason: job.cancelled,
      durationMs: elapsed(job),
    }) as GeometryBuildTerminal
  }
  const supersededBy = staleReplacement(request)
  if (supersededBy) {
    return jobEvent(request, {
      status: 'stale',
      phase,
      ...(execution ? { execution } : {}),
      supersededBy,
      durationMs: elapsed(job),
    }) as GeometryBuildTerminal
  }
  return null
}

async function runBuild(
  request: GeometryBuildRequest,
  planned: GeometryExecutionDescriptor,
) {
  const job: ActiveJob = {
    request,
    startedAt: performance.now(),
    cancelled: null,
    initialization: new AbortController(),
  }
  activeJobs.set(request.jobId, job)
  postEvent(jobEvent(request, { status: 'accepted', phase: 'queued' }))

  const alreadyStale = terminalState(job, 'queued', planned)
  if (alreadyStale) {
    postEvent(alreadyStale)
    activeJobs.delete(request.jobId)
    return
  }

  postEvent(jobEvent(request, { status: 'started', phase: 'initializing' }))

  let lastHeartbeat = performance.now()
  let completedExecution: GeometryExecutionDescriptor | undefined
  let phase: 'initializing' | 'compiling' | 'serializing' = 'initializing'
  try {
    // Cold WASM compilation belongs to initialization, not provider readiness.
    // Cancel only this wait; shared warmup can serve the next admitted job.
    await defaultGeometryBuildEngine.initializeSource(request.source, {
      quality: request.quality,
      purpose: request.quality,
    }, job.initialization.signal)
    const initializationTerminal = terminalState(job, phase, planned)
    if (initializationTerminal) { postEvent(initializationTerminal); return }
    phase = 'compiling'
    postEvent(jobEvent(request, { status: 'progress', phase, progress: null }))
    // The parser's top-level statement loop yields to the event loop
    // periodically, so queued cancel messages are delivered mid-parse and
    // shouldAbort stops a superseded/cancelled evaluation cooperatively — the
    // warm worker (and its cached WASM) survives. A single wedged statement
    // still cannot yield; BuildCoordinator's grace timer replaces the worker
    // in that case and remains the hard cancellation boundary.
    const built = await defaultGeometryBuildEngine.buildSource(request.source, {
      quality: request.quality,
      purpose: request.quality,
    }, {
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
    completedExecution = built.execution
    const result = built.result
    const terminal = terminalState(job, 'compiling', built.execution)
    if (terminal) {
      postEvent(terminal)
      return
    }

    phase = 'serializing'
    postEvent(jobEvent(request, { status: 'progress', phase, progress: 0.95 }))
    if (request.selectionSurfaces) {
      await warmGeometryKernel()
      const terminal = terminalState(job, 'serializing', built.execution)
      if (terminal) { postEvent(terminal); return }
      const meshes: typeof result.meshes = []
      let sliceStarted = performance.now()
      for (const mesh of result.meshes) {
        meshes.push(publishSelectionSurfaces(mesh))
        if (performance.now() - sliceStarted >= 16) {
          // Deliver queued cancellation between bounded synchronous Rust calls,
          // including after the final mesh, before publishing any buffers.
          await new Promise<void>(resolve => setTimeout(resolve, 0))
          const terminal = terminalState(job, 'serializing', built.execution)
          if (terminal) { postEvent(terminal); return }
          sliceStarted = performance.now()
          if (sliceStarted - lastHeartbeat >= HEARTBEAT_THROTTLE_MS) {
            lastHeartbeat = sliceStarted
            postEvent(jobEvent(request, {status: 'progress', phase: 'serializing', progress: null}))
          }
        }
      }
      result.meshes = meshes
    }
    const response = jobEvent(request, {
      status: 'succeeded',
      phase: 'complete',
      execution: built.execution,
      ...result,
      durationMs: elapsed(job),
    })
    // Validate the complete success packet while every ArrayBuffer is still
    // Worker-owned. In particular, a legacy identity longer than the frozen
    // 256-code-unit limit must never cross the boundary as an invalid success
    // (and must never be truncated or replaced with a synthetic alias).
    if (!isGeometryWorkerEvent(response)) {
      postEvent(jobEvent(request, {
        status: 'failed',
        phase: 'serializing',
        execution: built.execution,
        error: UNPUBLISHABLE_RESULT_ERROR,
        durationMs: elapsed(job),
      }))
      return
    }
    postEvent(response, meshTransferables(result.meshes))
  } catch (error) {
    const attachedExecution = geometryExecutionForError(error) ?? completedExecution
    const terminal = terminalState(job, phase, attachedExecution ?? plannedExecution(request))
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
        phase,
        ...(attachedExecution ? { execution: attachedExecution } : {}),
        reason: 'superseded',
        durationMs: elapsed(job),
      }))
      return
    }
    const execution = attachedExecution
      ?? (error instanceof GeometryEngineUnavailableError
      || error instanceof GeometryCapabilityUnavailableError
      ? error.execution
      : undefined)
    postEvent(jobEvent(request, {
      status: 'failed',
      phase,
      ...(execution ? { execution } : {}),
      error: serializedBuildError(error),
      durationMs: elapsed(job),
    }))
  } finally {
    activeJobs.delete(request.jobId)
  }
}

function acceptBuild(request: GeometryBuildRequest) {
  // A job id is admitted at most once for the entire Worker lifetime. The
  // high-water rule also rejects out-of-order lower ids, which cannot occur on
  // the legitimate FIFO Coordinator channel.
  if (request.jobId <= highestAcceptedJobId) return
  highestAcceptedJobId = request.jobId

  // Validate routing before touching revision/queue state. A malformed newer
  // request must not supersede valid work already running, and planning never
  // warms either provider.
  let planned: GeometryExecutionDescriptor
  const startedAt = performance.now()
  try {
    planned = defaultGeometryBuildEngine.planSource(request.source, {
      quality: request.quality,
      purpose: request.quality,
    })
  } catch (error) {
    postEvent(jobEvent(request, {
      status: 'failed',
      phase: 'queued',
      error: serializedBuildError(error),
      durationMs: performance.now() - startedAt,
    }))
    return
  }
  if (request.documentRevision > latestDocumentRevision) {
    latestDocumentRevision = request.documentRevision
    latestByQuality.clear()
  }
  if (request.documentRevision === latestDocumentRevision) {
    const previous = latestByQuality.get(request.quality)
    if (!previous || request.jobId > previous.jobId) latestByQuality.set(request.quality, request)
  }
  void runBuild(request, planned)
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
  job.initialization.abort()
}

self.addEventListener('message', (event: MessageEvent<unknown>) => {
  if (exactJobStarted) return
  // Exact Solid clients own a disposable instance of this same worker entry.
  // Their JSON result is separate from the display protocol and has no handles.
  if (event.data && typeof event.data === 'object' && 'kind' in event.data && event.data.kind === 'exact-solid') {
    if (highestAcceptedJobId !== -1 || !isExactSolidRequest(event.data)) return
    exactJobStarted = true
    void import('../services/solid/exactSolidRuntime')
      .then(({ runExactSolidRequest }) => runExactSolidRequest(event.data))
      .then(response => self.postMessage(response))
      .catch(() => self.postMessage({
        kind: 'exact-solid', version: 1, ok: false,
        error: { name: 'Error', message: 'Could not load exact-solid processing.' },
      }))
    return
  }
  // Validation happens before any lifecycle state changes, so a malformed ID
  // cannot poison later work. Every accepted build reaches an explicit
  // succeeded/failed/stale/cancelled terminal unless the worker is replaced.
  if (isGeometryWorkerRequest(event.data)) {
    if (event.data.type === 'build') acceptBuild(event.data)
    else cancelBuild(event.data)
  }
})
