import type { Worker } from 'node:worker_threads'
import { sha256Hex } from '../../core/sha256'
import {
  DIRECT_GEOMETRY_IDENTITY,
  DIRECT_GEOMETRY_PROTOCOL_VERSION,
  type DirectGeometryCancel,
  type DirectGeometryRequest,
} from '../directGeometryProtocol'
import {
  DirectGeometryJoinError,
  DirectGeometryProtocolError,
  DirectGeometryWorkerCrashError,
  MAX_DRAINED_CHILD_OUTPUT_BYTES,
  type ActiveJob,
  type AdmittedJob,
  type StopReason,
} from './errorsAndTypes'

export function attachBoundedOutputDrain(stream: Worker['stdout']): () => void {
  let countedBytes = 0
  const onData = (chunk: unknown) => {
    const byteLength =
      typeof chunk === 'string' ? Buffer.byteLength(chunk) : ArrayBuffer.isView(chunk) ? chunk.byteLength : 0
    countedBytes = Math.min(MAX_DRAINED_CHILD_OUTPUT_BYTES, countedBytes + byteLength)
  }
  try {
    stream.unpipe()
    stream.on('data', onData)
    stream.resume()
    return () => {
      try {
        stream.off('data', onData)
      } catch {
        /* cleanup is best effort */
      }
    }
  } catch {
    try {
      stream.resume()
    } catch {
      /* no-op */
    }
    return () => undefined
  }
}

export function abortError(message: string): DOMException {
  return new DOMException(message, 'AbortError')
}

export function buildWorkerRequest(job: AdmittedJob, workerEpoch: number): DirectGeometryRequest {
  const base = {
    protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
    workerEpoch,
    jobId: job.id,
    identity: DIRECT_GEOMETRY_IDENTITY,
  } as const
  return job.kind === 'build'
    ? Object.freeze({
        ...base,
        type: 'build' as const,
        source: job.source!,
        sourceSha256: sha256Hex(job.source!),
        quality: job.quality!,
        purpose: job.purpose!,
      })
    : Object.freeze({
        ...base,
        type: 'capabilities' as const,
        sourceSha256: null,
        quality: null,
        purpose: null,
      })
}

export function createActiveJobHandle(params: {
  job: AdmittedJob
  request: DirectGeometryRequest
  worker: Worker
  epoch: number
  startupTimeoutMs: number
  onMessage: (active: ActiveJob, value: unknown) => void
  onSettleFailure: (active: ActiveJob, error: Error) => void
  onStartupTimeout: (active: ActiveJob) => void
}): ActiveJob {
  const { job, request, worker, epoch } = params
  let resolveDone!: () => void
  const done = new Promise<void>(resolve => {
    resolveDone = resolve
  })
  const stopStdout = attachBoundedOutputDrain(worker.stdout)
  const stopStderr = attachBoundedOutputDrain(worker.stderr)
  const active: ActiveJob = {
    job,
    request,
    worker,
    epoch,
    done,
    resolveDone,
    stopOutputDrains: () => {
      stopStdout()
      stopStderr()
    },
    detachWorkerListeners: () => undefined,
    started: false,
    settling: false,
    stopReason: null,
    startupTimer: undefined,
    graceTimer: undefined,
  }
  const onMessage = (value: unknown) => params.onMessage(active, value)
  const onMessageError = (cause: Error) => {
    params.onSettleFailure(
      active,
      new DirectGeometryProtocolError(epoch, `Worker message deserialization failed: ${cause.name}`),
    )
  }
  const onError = (cause: Error) => {
    params.onSettleFailure(active, new DirectGeometryWorkerCrashError(epoch, { cause }))
  }
  const onExit = () => {
    params.onSettleFailure(active, new DirectGeometryWorkerCrashError(epoch))
  }
  active.detachWorkerListeners = () => {
    worker.off('message', onMessage)
    worker.off('messageerror', onMessageError)
    worker.off('error', onError)
    worker.off('exit', onExit)
  }
  worker.on('message', onMessage)
  worker.on('messageerror', onMessageError)
  worker.on('error', onError)
  worker.on('exit', onExit)
  active.startupTimer = setTimeout(() => params.onStartupTimeout(active), params.startupTimeoutMs)
  return active
}

export function sendCancelToWorker(active: ActiveJob, reason: StopReason): void {
  const cancel: DirectGeometryCancel = Object.freeze({
    protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
    type: 'cancel',
    workerEpoch: active.request.workerEpoch,
    jobId: active.request.jobId,
    sourceSha256: active.request.sourceSha256,
    quality: active.request.quality,
    purpose: active.request.purpose,
    reason: reason === 'closed' ? 'closed' : reason === 'cancelled' ? 'cancelled' : 'deadline',
    identity: DIRECT_GEOMETRY_IDENTITY,
  })
  try {
    active.worker.postMessage(cancel)
  } catch {
    /* The child is still hard-terminated and joined below. */
  }
}

export async function terminateAndJoinWorker(
  active: ActiveJob,
  joinTimeoutMs: number,
): Promise<DirectGeometryJoinError | null> {
  type JoinOutcome = { kind: 'joined' } | { kind: 'failed'; cause: unknown } | { kind: 'timeout' }
  let joinTimer: ReturnType<typeof setTimeout> | undefined
  const termination = Promise.resolve()
    .then(() => active.worker.terminate())
    .then<JoinOutcome, JoinOutcome>(
      () => ({ kind: 'joined' }),
      cause => ({ kind: 'failed', cause }),
    )
  const outcome = await Promise.race<JoinOutcome>([
    termination,
    new Promise(resolve => {
      joinTimer = setTimeout(() => resolve({ kind: 'timeout' }), joinTimeoutMs)
    }),
  ])
  if (joinTimer !== undefined) clearTimeout(joinTimer)
  if (outcome.kind !== 'joined') {
    try {
      active.worker.unref()
    } catch {
      /* injected doubles may omit unref */
    }
    try {
      active.stopOutputDrains()
      active.worker.stdout.destroy()
      active.worker.stderr.destroy()
    } catch {
      /* Quarantine and Worker.unref() remain the authoritative containment. */
    }
    if (outcome.kind === 'timeout') {
      void termination.then(lateOutcome => {
        if (lateOutcome.kind !== 'joined') return
        try {
          active.stopOutputDrains()
          active.detachWorkerListeners()
        } catch {
          /* The realm is already joined; late cleanup is best effort. */
        }
      })
    }
    return new DirectGeometryJoinError(
      active.epoch,
      joinTimeoutMs,
      outcome.kind === 'failed' ? { cause: outcome.cause } : {},
    )
  }
  active.stopOutputDrains()
  active.detachWorkerListeners()
  return null
}
