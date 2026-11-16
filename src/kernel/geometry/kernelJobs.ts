/**
 * Host driver for the kernel's unified long-job protocol.
 *
 * `start` returns a job handle without doing the heavy work, `step(handle,
 * budget)` advances at most `budget` cooperative units (one pairwise Boolean,
 * one render batch) and returns progress or the result, and `cancel(handle)`
 * frees every unfinished allocation. The driver awaits `checkpoint` before
 * every step; in the geometry Worker that checkpoint yields to the message
 * loop and throws once a cancel message has arrived, so cancellation is
 * observed within one step and the Worker itself is never replaced.
 */
import { copyBuffer } from './bufferTransport'
import { GeometryKernelError, decodeNurbsResult, kernelRuntime } from './kernel'
import { copyRenderMesh, type KernelRenderMesh } from './meshAnalysis'

export type KernelJobKind = 'mesh-boolean' | 'render-prepare'

export interface KernelJobProgress {
  readonly kind: KernelJobKind
  readonly done: number
  readonly total: number
  /** Cooperative units polled so far, including the completing one. */
  readonly steps: number
}

export type KernelJobCheckpoint = (progress: KernelJobProgress | null) => Promise<void>

type KernelJobStep =
  | { state: 'pending'; kind: KernelJobKind; done: number; total: number; steps: number }
  | { state: 'complete'; kind: KernelJobKind; result: number; steps: number }

export const KERNEL_BOOLEAN_OPERATIONS = Object.freeze({ union: 0, difference: 1, intersection: 2 } as const)
export type KernelBooleanOperation = keyof typeof KERNEL_BOOLEAN_OPERATIONS

let latestProgress: KernelJobProgress | null = null

/** Most recent pending progress of any driven job, for liveness heartbeats. */
export function latestKernelJobProgress(): KernelJobProgress | null {
  return latestProgress
}

function isStep(value: unknown): value is KernelJobStep {
  if (!value || typeof value !== 'object') return false
  const step = value as Record<string, unknown>
  if (step.kind !== 'mesh-boolean' && step.kind !== 'render-prepare') return false
  if (!Number.isSafeInteger(step.steps)) return false
  if (step.state === 'complete') return Number.isSafeInteger(step.result)
  return step.state === 'pending' && Number.isSafeInteger(step.done) && Number.isSafeInteger(step.total)
}

/**
 * Drive one started job to completion. The handle is cancelled on every exit
 * that does not consume it (checkpoint rejection, decode failure), so no
 * kernel scratch outlives an abandoned job.
 */
export async function driveKernelJob(
  start: () => number,
  checkpoint: KernelJobCheckpoint,
  budget = 1,
): Promise<{ kind: KernelJobKind; result: number; steps: number }> {
  if (!Number.isInteger(budget) || budget < 1)
    throw new GeometryKernelError('GEOMETRY_INVALID_INPUT', 'Invalid kernel job budget')
  const { exports: wasm, takeResponse } = kernelRuntime()
  let job = start()
  let progress: KernelJobProgress | null = null
  try {
    for (;;) {
      await checkpoint(progress)
      const step = decodeNurbsResult<unknown>(takeResponse(wasm.abi_job_step(job, budget)))
      if (!isStep(step)) throw new GeometryKernelError('GEOMETRY_PROTOCOL', 'Malformed kernel job step')
      if (step.state === 'complete') {
        job = 0
        latestProgress = null
        return { kind: step.kind, result: step.result, steps: step.steps }
      }
      progress = Object.freeze({ kind: step.kind, done: step.done, total: step.total, steps: step.steps })
      latestProgress = progress
    }
  } catch (error) {
    // A failed step already consumed its job; cancelling a dead handle is a no-op.
    latestProgress = null
    throw error
  } finally {
    if (job) wasm.abi_job_cancel(job)
  }
}

/** Cooperative n-ary mesh Boolean; resolves to the new retained solid handle. */
export async function booleanSolidsCooperativelyInKernel(
  ids: readonly number[],
  operation: KernelBooleanOperation,
  checkpoint: KernelJobCheckpoint,
  budget = 1,
): Promise<number> {
  if (ids.length === 0 || ids.some(id => !Number.isInteger(id) || id < 1 || id > 0xffffffff)) {
    throw new GeometryKernelError('GEOMETRY_INVALID_INPUT', 'Invalid CAD handle')
  }
  const op = KERNEL_BOOLEAN_OPERATIONS[operation]
  if (op === undefined) throw new GeometryKernelError('GEOMETRY_INVALID_INPUT', 'Invalid Boolean operation')
  const { exports: wasm, takeResponse } = kernelRuntime()
  const handles = new Uint32Array(ids)
  const { result } = await driveKernelJob(
    () => {
      const ptr = copyBuffer(wasm, new Uint8Array(handles.buffer))
      try {
        return decodeNurbsResult<number>(takeResponse(wasm.abi_job_boolean_start(ptr, handles.length, op)))
      } finally {
        wasm.abi_free(ptr, handles.byteLength)
      }
    },
    checkpoint,
    budget,
  )
  return result
}

/** Cooperative counterpart of `renderMeshInKernel`; output is bit-identical. */
export async function renderMeshCooperativelyInKernel(
  id: number,
  creaseCosine: number,
  checkpoint: KernelJobCheckpoint,
  budget = 1,
): Promise<KernelRenderMesh> {
  if (!Number.isInteger(id) || id < 1 || id > 0xffffffff)
    throw new GeometryKernelError('GEOMETRY_INVALID_INPUT', 'Invalid CAD handle')
  if (!Number.isFinite(creaseCosine)) throw new GeometryKernelError('GEOMETRY_INVALID_INPUT', 'Invalid crease cosine')
  const { exports: wasm, memory, takeResponse } = kernelRuntime()
  const { result: handle } = await driveKernelJob(
    () => decodeNurbsResult<number>(takeResponse(wasm.abi_job_render_start(id, creaseCosine))),
    checkpoint,
    budget,
  )
  try {
    // Copied before any await: the view must not outlive a memory.grow.
    return copyRenderMesh(wasm, memory.buffer, handle)
  } finally {
    wasm.abi_array_free(handle)
  }
}
