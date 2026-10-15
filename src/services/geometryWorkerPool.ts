/**
 * Hardware Multithreading & Shared Memory Worker Pool.
 *
 * Implements multi-core CPU parallelization for independent CSG branches,
 * batch B-Rep evaluations, and parallel mesh kernels:
 * 1. Hardware concurrency detection (scales up to 16 cores on Apple M-series / modern CPUs).
 * 2. Cross-Origin Isolation & SharedArrayBuffer capability probing.
 * 3. Shared WebAssembly.Memory factory for Wasm Threads (+atomics, +bulk-memory).
 * 4. Work-stealing / least-loaded task dispatcher with Transferable zero-copy extraction
 *    and AbortSignal cancellation.
 */

export interface HardwareConcurrencyCapabilities {
  /** Number of logical CPU cores reported by the platform */
  logicalCores: number
  /** Recommended worker pool size (clamped to [1..maxWorkers]) */
  recommendedWorkers: number
  /** Whether globalThis.crossOriginIsolated is true */
  crossOriginIsolated: boolean
  /** Whether SharedArrayBuffer is available and constructible */
  sharedArrayBufferSupported: boolean
  /** Whether Atomics.wait / notify are available */
  atomicsSupported: boolean
  /** Whether WebAssembly shared memory can be instantiated */
  wasmSharedMemorySupported: boolean
}

let cachedSharedMemCaps: {
  crossOriginIsolated: boolean
  sharedArrayBufferSupported: boolean
  atomicsSupported: boolean
  wasmSharedMemorySupported: boolean
  sabCtor: typeof SharedArrayBuffer | undefined
} | null = null

function getSharedMemoryCapabilities() {
  const currentSabCtor = typeof SharedArrayBuffer !== 'undefined' ? SharedArrayBuffer : undefined
  const crossOriginIsolated =
    typeof globalThis !== 'undefined' && (globalThis as { crossOriginIsolated?: boolean }).crossOriginIsolated === true
  if (cachedSharedMemCaps && cachedSharedMemCaps.sabCtor === currentSabCtor && cachedSharedMemCaps.crossOriginIsolated === crossOriginIsolated) {
    return cachedSharedMemCaps
  }

  let sharedArrayBufferSupported = false
  try {
    if (currentSabCtor !== undefined) {
      const sab = new currentSabCtor(16)
      sharedArrayBufferSupported = sab.byteLength === 16
    }
  } catch {
    sharedArrayBufferSupported = false
  }

  const atomicsSupported =
    typeof Atomics !== 'undefined' &&
    typeof Atomics.add === 'function' &&
    typeof Atomics.load === 'function'

  let wasmSharedMemorySupported = false
  if (sharedArrayBufferSupported && typeof WebAssembly !== 'undefined' && typeof WebAssembly.Memory === 'function') {
    try {
      const mem = new WebAssembly.Memory({ initial: 1, maximum: 2, shared: true })
      wasmSharedMemorySupported = mem.buffer instanceof currentSabCtor!
    } catch {
      wasmSharedMemorySupported = false
    }
  }

  cachedSharedMemCaps = {
    crossOriginIsolated,
    sharedArrayBufferSupported,
    atomicsSupported,
    wasmSharedMemorySupported,
    sabCtor: currentSabCtor,
  }
  return cachedSharedMemCaps
}

/**
 * Probes the current runtime environment for hardware multithreading and shared memory support.
 */
export function detectHardwareConcurrency(maxWorkers = 16): HardwareConcurrencyCapabilities {
  const rawCores =
    typeof navigator !== 'undefined' && typeof navigator.hardwareConcurrency === 'number'
      ? navigator.hardwareConcurrency
      : 4

  const logicalCores = Math.max(1, rawCores)
  const recommendedWorkers = Math.max(1, Math.min(maxWorkers, Math.max(1, logicalCores - 1)))
  const sharedCaps = getSharedMemoryCapabilities()

  return {
    logicalCores,
    recommendedWorkers,
    crossOriginIsolated: sharedCaps.crossOriginIsolated,
    sharedArrayBufferSupported: sharedCaps.sharedArrayBufferSupported,
    atomicsSupported: sharedCaps.atomicsSupported,
    wasmSharedMemorySupported: sharedCaps.wasmSharedMemorySupported,
  }
}

/**
 * Creates a shared WebAssembly.Memory (64 KiB per page) when supported,
 * or falls back to standard linear WebAssembly.Memory.
 */
export function createKernelWasmMemory(
  initialPages = 256, // 16 MiB
  maximumPages = 16384, // 1 GiB
  preferShared = true
): { memory: WebAssembly.Memory; isShared: boolean } {
  if (preferShared) {
    try {
      const memory = new WebAssembly.Memory({
        initial: initialPages,
        maximum: maximumPages,
        shared: true,
      })
      return { memory, isShared: true }
    } catch {
      // Fallback to unshared memory if COOP/COEP headers are absent
    }
  }

  const memory = new WebAssembly.Memory({
    initial: initialPages,
    maximum: maximumPages,
    shared: false,
  })
  return { memory, isShared: false }
}

/**
 * Extracts unique ArrayBuffers from typed arrays for zero-copy Worker postMessage transfer.
 */
export function collectTransferableBuffers(payload: unknown): ArrayBuffer[] {
  const seen = new Set<ArrayBuffer>()

  function visit(val: unknown) {
    if (!val || typeof val !== 'object') return

    if (ArrayBuffer.isView(val)) {
      const buf = val.buffer
      // SharedArrayBuffer cannot and need not be in the transfer list
      if (buf instanceof ArrayBuffer && !seen.has(buf)) {
        seen.add(buf)
      }
      return
    }

    if (val instanceof ArrayBuffer) {
      if (!seen.has(val)) seen.add(val)
      return
    }

    if (Array.isArray(val)) {
      for (const item of val) visit(item)
      return
    }

    for (const key of Object.keys(val as Record<string, unknown>)) {
      visit((val as Record<string, unknown>)[key])
    }
  }

  visit(payload)
  return Array.from(seen)
}

export interface WorkerTaskOptions {
  priority?: number
  signal?: AbortSignal
  transfer?: boolean
}

export type TaskExecutor<TInput, TOutput> = (input: TInput, workerIndex: number) => Promise<TOutput> | TOutput

interface QueuedTask<TInput, TOutput> {
  id: number
  input: TInput
  priority: number
  signal?: AbortSignal
  aborted?: boolean
  resolve: (value: TOutput) => void
  reject: (reason: unknown) => void
}

export interface WorkerPoolStats {
  poolSize: number
  activeTasks: number
  queuedTasks: number
  completedTasks: number
  perWorkerCompleted: number[]
}

/**
 * Parallel Work-Stealing / Least-Loaded Scheduler for multi-core geometry evaluation.
 */
export class GeometryWorkerPool<TInput, TOutput> {
  readonly poolSize: number
  private readonly executor: TaskExecutor<TInput, TOutput>
  private readonly busyWorkers: Uint8Array
  private activeCount = 0
  private readonly perWorkerCount: number[]
  private readonly queue: QueuedTask<TInput, TOutput>[] = []
  private queueHead = 0
  private activeQueuedCount = 0
  private nextTaskId = 1
  private completedCount = 0
  private disposed = false

  constructor(executor: TaskExecutor<TInput, TOutput>, poolSize?: number) {
    const caps = detectHardwareConcurrency()
    this.poolSize = Math.max(1, poolSize ?? caps.recommendedWorkers)
    this.executor = executor
    this.busyWorkers = new Uint8Array(this.poolSize)
    this.perWorkerCount = new Array(this.poolSize).fill(0)
  }

  /**
   * Submits a single task to the parallel pool.
   */
  submit(input: TInput, options: WorkerTaskOptions = {}): Promise<TOutput> {
    if (this.disposed) {
      return Promise.reject(new Error('GeometryWorkerPool has been disposed'))
    }
    if (options.signal?.aborted) {
      return Promise.reject(new Error('Task aborted'))
    }

    return new Promise<TOutput>((resolve, reject) => {
      const task: QueuedTask<TInput, TOutput> = {
        id: this.nextTaskId++,
        input,
        priority: options.priority ?? 0,
        signal: options.signal,
        resolve,
        reject,
      }

      if (options.signal) {
        options.signal.addEventListener(
          'abort',
          () => {
            if (!task.aborted) {
              task.aborted = true
              this.activeQueuedCount--
              reject(new Error('Task aborted'))
            }
          },
          { once: true }
        )
      }

      // Insert in priority order (higher priority first, FIFO for ties) using binary search
      const q = this.queue
      const len = q.length
      if (len === this.queueHead || task.priority <= q[len - 1]!.priority) {
        q.push(task)
      } else {
        let lo = this.queueHead
        let hi = len
        while (lo < hi) {
          const mid = (lo + hi) >>> 1
          if (task.priority > q[mid]!.priority) {
            hi = mid
          } else {
            lo = mid + 1
          }
        }
        q.splice(lo, 0, task)
      }
      this.activeQueuedCount++

      this.pump()
    })
  }

  /**
   * Evaluates a batch of independent tasks in parallel across all available worker slots.
   */
  async evaluateBatch(inputs: readonly TInput[], options: WorkerTaskOptions = {}): Promise<TOutput[]> {
    return Promise.all(inputs.map(input => this.submit(input, options)))
  }

  private findAvailableWorker(): number | null {
    if (this.activeCount >= this.poolSize) return null
    // Choose idle worker with the fewest completed tasks for even load balancing
    let bestWorker: number | null = null
    let bestCount = Infinity

    for (let w = 0; w < this.poolSize; w++) {
      if (this.busyWorkers[w] === 0) {
        const count = this.perWorkerCount[w]!
        if (count < bestCount) {
          bestCount = count
          bestWorker = w
        }
      }
    }
    return bestWorker
  }

  private dequeueTask(): QueuedTask<TInput, TOutput> | null {
    while (this.queueHead < this.queue.length) {
      const task = this.queue[this.queueHead]!
      this.queue[this.queueHead] = undefined as unknown as QueuedTask<TInput, TOutput>
      this.queueHead++
      if (this.queueHead === this.queue.length) {
        this.queue.length = 0
        this.queueHead = 0
      } else if (this.queueHead >= 64 && this.queueHead * 2 >= this.queue.length) {
        this.queue.splice(0, this.queueHead)
        this.queueHead = 0
      }
      if (task.aborted) continue
      this.activeQueuedCount--
      return task
    }
    return null
  }

  private pump(): void {
    if (this.disposed) return

    while (this.activeQueuedCount > 0 && this.activeCount < this.poolSize) {
      const workerIdx = this.findAvailableWorker()
      if (workerIdx === null) break

      const task = this.dequeueTask()
      if (!task) break
      if (task.signal?.aborted) {
        task.aborted = true
        task.reject(new Error('Task aborted'))
        continue
      }

      // Mark aborted=true so the queued-abort listener becomes a no-op once dispatched
      task.aborted = true
      this.busyWorkers[workerIdx] = 1
      this.activeCount++

      Promise.resolve()
        .then(() => this.executor(task.input, workerIdx))
        .then(
          result => {
            this.busyWorkers[workerIdx] = 0
            this.activeCount--
            this.perWorkerCount[workerIdx]!++
            this.completedCount++
            task.resolve(result)
            this.pump()
          },
          err => {
            this.busyWorkers[workerIdx] = 0
            this.activeCount--
            task.reject(err)
            this.pump()
          }
        )
    }
  }

  getStats(): WorkerPoolStats {
    return {
      poolSize: this.poolSize,
      activeTasks: this.activeCount,
      queuedTasks: this.activeQueuedCount,
      completedTasks: this.completedCount,
      perWorkerCompleted: [...this.perWorkerCount],
    }
  }

  dispose(): void {
    this.disposed = true
    while (true) {
      const task = this.dequeueTask()
      if (!task) break
      task.aborted = true
      task.reject(new Error('GeometryWorkerPool disposed'))
    }
  }
}
