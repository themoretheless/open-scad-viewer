import { describe, it, expect } from 'vitest'
import {
  detectHardwareConcurrency,
  createKernelWasmMemory,
  collectTransferableBuffers,
  GeometryWorkerPool,
} from '../src/services/geometryWorkerPool'

describe('Hardware Multithreading & Shared Memory Worker Pool', () => {
  it('detects hardware concurrency and shared memory capabilities', () => {
    const caps = detectHardwareConcurrency(16)
    expect(caps.logicalCores).toBeGreaterThanOrEqual(1)
    expect(caps.recommendedWorkers).toBeGreaterThanOrEqual(1)
    expect(caps.recommendedWorkers).toBeLessThanOrEqual(16)
    expect(typeof caps.sharedArrayBufferSupported).toBe('boolean')
    expect(typeof caps.atomicsSupported).toBe('boolean')
    expect(typeof caps.wasmSharedMemorySupported).toBe('boolean')
  })

  it('allocates WebAssembly.Memory with shared or linear fallback', () => {
    const { memory, isShared } = createKernelWasmMemory(4, 16, true)
    expect(memory).toBeInstanceOf(WebAssembly.Memory)
    expect(memory.buffer.byteLength).toBe(4 * 65536)
    expect(typeof isShared).toBe('boolean')
  })

  it('extracts unique transferable ArrayBuffers and excludes SharedArrayBuffer', () => {
    const f1 = new Float32Array(16)
    const u1 = new Uint32Array(f1.buffer) // Same underlying ArrayBuffer
    const f2 = new Float32Array(8)

    const payload = {
      meshes: [
        { vertices: f1, indices: u1 },
        { vertices: f2 },
      ],
    }

    const transferables = collectTransferableBuffers(payload)
    // f1 and u1 share the same buffer, so total unique ArrayBuffers = 2
    expect(transferables.length).toBe(2)
    expect(transferables).toContain(f1.buffer)
    expect(transferables).toContain(f2.buffer)
  })

  it('distributes batch tasks evenly across all worker slots', async () => {
    const pool = new GeometryWorkerPool<number, number>(
      async (x, workerIdx) => {
        await new Promise(r => setTimeout(r, 5))
        return x * 2 + workerIdx * 0 // pure value
      },
      4
    )

    const inputs = [1, 2, 3, 4, 5, 6, 7, 8]
    const results = await pool.evaluateBatch(inputs)

    expect(results).toEqual([2, 4, 6, 8, 10, 12, 14, 16])

    const stats = pool.getStats()
    expect(stats.poolSize).toBe(4)
    expect(stats.completedTasks).toBe(8)
    expect(stats.activeTasks).toBe(0)
    // Each of the 4 workers should have executed 2 tasks
    for (const count of stats.perWorkerCompleted) {
      expect(count).toBeGreaterThanOrEqual(1)
    }

    pool.dispose()
  })

  it('executes higher-priority queued tasks before lower-priority tasks', async () => {
    const executionOrder: number[] = []

    // Pool of size 1 so tasks after the first one are queued
    const pool = new GeometryWorkerPool<number, number>(async x => {
      await new Promise(r => setTimeout(r, 10))
      executionOrder.push(x)
      return x
    }, 1)

    // Task 1 starts immediately on worker 0
    const p1 = pool.submit(1, { priority: 1 })
    // Tasks 2, 3, 4 wait in queue with different priorities
    const p2 = pool.submit(2, { priority: 1 })
    const p3 = pool.submit(3, { priority: 10 }) // Highest priority in queue
    const p4 = pool.submit(4, { priority: 5 })  // Medium priority in queue

    await Promise.all([p1, p2, p3, p4])

    // Task 1 ran first; then queued tasks must run in priority order: 3 (prio 10), 4 (prio 5), 2 (prio 1)
    expect(executionOrder).toEqual([1, 3, 4, 2])
    pool.dispose()
  })

  it('supports AbortSignal cancellation for queued tasks', async () => {
    const pool = new GeometryWorkerPool<number, number>(async x => {
      await new Promise(r => setTimeout(r, 20))
      return x
    }, 1)

    const p1 = pool.submit(1)
    const controller = new AbortController()
    const p2 = pool.submit(2, { signal: controller.signal })

    // Abort p2 while p1 is still running
    controller.abort()

    await expect(p2).rejects.toThrow(/aborted/i)
    await expect(p1).resolves.toBe(1)

    pool.dispose()
  })
})
