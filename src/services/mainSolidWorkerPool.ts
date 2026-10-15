import { detectHardwareConcurrency, type WorkerPoolStats } from './geometryWorkerPool'
import type { MainSolidJob, MainSolidResults } from './mainSolidProtocol'
import {
  MainSolidWorkerClient,
  MainSolidWorkerError,
  type MainSolidPort,
  type MainSolidRunOptions,
} from './mainSolidWorkerClient'

export interface MainSolidPoolRunOptions extends MainSolidRunOptions {
  /** Higher priority tasks are dispatched ahead of lower priority queued tasks (default: 0) */
  priority?: number
}

interface QueuedSolidTask<K extends MainSolidJob['kind'] = MainSolidJob['kind']> {
  id: number
  job: Extract<MainSolidJob, { kind: K }>
  options: MainSolidPoolRunOptions
  priority: number
  aborted?: boolean
  resolve: (value: MainSolidResults[K]) => void
  reject: (reason: unknown) => void
  cleanupAbort?: () => void
}

/**
 * Hardware-concurrency-aware Multi-Worker Pool for `MainSolidJob` operations.
 *
 * Unlike a single `MainSolidWorkerClient` (which aborts and terminates its worker
 * when a concurrent request arrives), `MainSolidWorkerPool` maintains up to `poolSize`
 * independent warm worker clients (`MainSolidWorkerClient` slots) and dispatches
 * concurrent requests in parallel across idle workers, queuing overflow requests
 * in priority-FIFO order without killing in-flight calculations.
 */
export class MainSolidWorkerPool {
  readonly poolSize: number
  private readonly slots: (MainSolidWorkerClient | null)[]
  private readonly busySlots: Uint8Array
  private activeSlotCount = 0
  private readonly perWorkerCompleted: number[]
  private readonly queue: QueuedSolidTask<any>[] = []
  private queueHead = 0
  private activeQueuedCount = 0
  private nextTaskId = 1
  private completedTasks = 0
  private closed = false

  constructor(
    private readonly portFactory: () => MainSolidPort,
    poolSize?: number
  ) {
    const caps = detectHardwareConcurrency(8)
    this.poolSize = Math.max(1, poolSize ?? Math.min(4, caps.recommendedWorkers))
    this.slots = new Array(this.poolSize).fill(null)
    this.busySlots = new Uint8Array(this.poolSize)
    this.perWorkerCompleted = new Array(this.poolSize).fill(0)
  }

  private getOrCreateSlot(index: number): MainSolidWorkerClient {
    let client = this.slots[index]
    if (!client) {
      client = new MainSolidWorkerClient(this.portFactory)
      this.slots[index] = client
    }
    return client
  }

  /**
   * Dispatches a `MainSolidJob` to an available worker in the pool, or queues it
   * if all workers are currently busy. Supports per-task `AbortSignal` and `timeoutMs`
   * without cancelling unrelated concurrent jobs on other workers.
   */
  run<K extends MainSolidJob['kind']>(
    job: Extract<MainSolidJob, { kind: K }>,
    options: MainSolidPoolRunOptions = {}
  ): Promise<MainSolidResults[K]> {
    if (this.closed) {
      return Promise.reject(new MainSolidWorkerError('CAD_DISPOSED', 'CAD worker pool is closed'))
    }
    if (options.signal?.aborted) {
      return Promise.reject(new MainSolidWorkerError('CAD_CANCELLED', 'Operation cancelled', 'AbortError'))
    }
    const timeoutMs = options.timeoutMs ?? 120000
    if (!Number.isFinite(timeoutMs) || timeoutMs < 1 || timeoutMs > 120000) {
      return Promise.reject(new RangeError('CAD timeout must be between 1 and 120000 ms'))
    }

    return new Promise<MainSolidResults[K]>((resolve, reject) => {
      const task: QueuedSolidTask<K> = {
        id: this.nextTaskId++,
        job,
        options,
        priority: options.priority ?? 0,
        resolve,
        reject,
      }

      if (options.signal) {
        const onAbort = () => {
          if (!task.aborted) {
            task.aborted = true
            this.activeQueuedCount--
            task.cleanupAbort?.()
            reject(new MainSolidWorkerError('CAD_CANCELLED', 'Operation cancelled', 'AbortError'))
          }
        }
        options.signal.addEventListener('abort', onAbort, { once: true })
        task.cleanupAbort = () => options.signal?.removeEventListener('abort', onAbort)
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
   * Executes multiple independent `MainSolidJob` requests in parallel across the worker pool.
   */
  runBatch<K extends MainSolidJob['kind']>(
    jobs: readonly Extract<MainSolidJob, { kind: K }>[],
    options: MainSolidPoolRunOptions = {}
  ): Promise<MainSolidResults[K][]> {
    return Promise.all(jobs.map(job => this.run(job, options)))
  }

  private findAvailableSlot(): number | null {
    if (this.activeSlotCount >= this.poolSize) return null
    // Prefer an already-warm idle slot with the fewest completed tasks, then a cold slot
    let bestWarm: number | null = null
    let bestWarmCount = Infinity
    let firstCold: number | null = null

    for (let i = 0; i < this.poolSize; i++) {
      if (this.busySlots[i] !== 0) continue
      if (this.slots[i] !== null) {
        const count = this.perWorkerCompleted[i]!
        if (count < bestWarmCount) {
          bestWarmCount = count
          bestWarm = i
        }
      } else if (firstCold === null) {
        firstCold = i
      }
    }

    return bestWarm ?? firstCold
  }

  private dequeueTask(): QueuedSolidTask<any> | null {
    while (this.queueHead < this.queue.length) {
      const task = this.queue[this.queueHead]!
      this.queue[this.queueHead] = undefined as unknown as QueuedSolidTask<any>
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
    if (this.closed) return

    while (this.activeQueuedCount > 0 && this.activeSlotCount < this.poolSize) {
      const slotIdx = this.findAvailableSlot()
      if (slotIdx === null) break

      const task = this.dequeueTask()
      if (!task) break
      task.cleanupAbort?.()

      if (task.options.signal?.aborted) {
        task.aborted = true
        task.reject(new MainSolidWorkerError('CAD_CANCELLED', 'Operation cancelled', 'AbortError'))
        continue
      }

      task.aborted = true
      this.busySlots[slotIdx] = 1
      this.activeSlotCount++
      const client = this.getOrCreateSlot(slotIdx)

      client
        .run(task.job, {
          signal: task.options.signal,
          timeoutMs: task.options.timeoutMs,
        })
        .then(
          result => {
            this.busySlots[slotIdx] = 0
            this.activeSlotCount--
            this.perWorkerCompleted[slotIdx]!++
            this.completedTasks++
            task.resolve(result)
            this.pump()
          },
          error => {
            this.busySlots[slotIdx] = 0
            this.activeSlotCount--
            task.reject(error)
            this.pump()
          }
        )
    }
  }

  /**
   * Cancels all queued and currently active jobs across all worker slots.
   */
  cancelAll(): void {
    while (true) {
      const task = this.dequeueTask()
      if (!task) break
      task.aborted = true
      task.cleanupAbort?.()
      task.reject(new MainSolidWorkerError('CAD_CANCELLED', 'Operation cancelled', 'AbortError'))
    }
    for (const slot of this.slots) {
      slot?.cancel()
    }
  }

  /**
   * Disposes all worker slots and rejects any remaining queued tasks.
   */
  dispose(): void {
    this.closed = true
    this.cancelAll()
    for (let i = 0; i < this.slots.length; i++) {
      this.slots[i]?.dispose()
      this.slots[i] = null
    }
    this.busySlots.fill(0)
    this.activeSlotCount = 0
  }

  getStats(): WorkerPoolStats {
    return {
      poolSize: this.poolSize,
      activeTasks: this.activeSlotCount,
      queuedTasks: this.activeQueuedCount,
      completedTasks: this.completedTasks,
      perWorkerCompleted: [...this.perWorkerCompleted],
    }
  }
}
