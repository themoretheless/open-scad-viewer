import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

interface MemorySample {
  readonly job: number
  readonly rss: number
  readonly heapUsed: number
  readonly external: number
  readonly arrayBuffers: number
}

interface ProbeRecord {
  readonly schema: 'mcp-manifold-plan-memory-probe'
  readonly version: 1
  readonly warmupJobs: number
  readonly measuredJobs: number
  readonly supervisor: {
    readonly activeWorkerEpoch: number | null
    readonly workersStarted: number
    readonly workersJoined: number
    readonly quarantined: boolean
  }
  readonly samples: readonly MemorySample[]
}

const probePath = fileURLToPath(
  new URL('./fixtures/mcp-manifold-plan-memory-probe.ts', import.meta.url),
)
const repositoryRoot = fileURLToPath(new URL('../', import.meta.url))
// 28 sequential worker jobs took about 51 s on a 4-core container, close to the old 60 s cap;
// this bounds runaway probes, not memory, which the budgets below check.
const probeTimeoutMs = 120_000

// The probe starts one worker thread per job. Under glibc's default allocator each thread
// gets its own malloc arena, and arenas freed on thread exit are not always returned to the
// OS, so RSS can grow by tens of MiB with flat JS heap/external/ArrayBuffer usage. CI pins
// these for the whole job (.github/workflows/ci.yml); pin them here so local runs measure the same.
const probeAllocatorEnv = {
  MALLOC_ARENA_MAX: '2',
  MALLOC_TRIM_THRESHOLD_: '0',
  MALLOC_MMAP_THRESHOLD_: '131072',
}

function slope(samples: readonly MemorySample[], field: keyof Omit<MemorySample, 'job'>): number {
  const count = samples.length
  const meanX = (count - 1) / 2
  const meanY = samples.reduce((sum, sample) => sum + sample[field], 0) / count
  let covariance = 0
  let variance = 0
  for (let index = 0; index < count; index++) {
    const centeredX = index - meanX
    covariance += centeredX * (samples[index][field] - meanY)
    variance += centeredX * centeredX
  }
  return covariance / variance
}

function runProbe(): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [
      '--expose-gc', '--import', 'tsx', probePath,
    ], {
      cwd: repositoryRoot,
      env: { ...process.env, ...probeAllocatorEnv },
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    let stdout = ''
    let stderr = ''
    child.stdout.setEncoding('utf8')
    child.stderr.setEncoding('utf8')
    child.stdout.on('data', chunk => { stdout += chunk })
    child.stderr.on('data', chunk => { stderr += chunk })
    const timeout = setTimeout(() => {
      child.kill()
      reject(new Error(`Memory probe exceeded ${probeTimeoutMs / 1000} seconds\n${stderr}`))
    }, probeTimeoutMs)
    child.once('error', error => {
      clearTimeout(timeout)
      reject(error)
    })
    child.once('close', code => {
      clearTimeout(timeout)
      if (code !== 0) reject(new Error(`Memory probe exited ${code}\n${stderr}`))
      else resolve({ stdout, stderr })
    })
  })
}

describe('MCP Manifold qualification child memory slope', () => {
  it('stays within the predeclared post-join retention budget', async () => {
    const run = await runProbe()
    expect(run.stderr).toBe('')
    const record = JSON.parse(run.stdout) as ProbeRecord
    expect(record).toMatchObject({
      schema: 'mcp-manifold-plan-memory-probe',
      version: 1,
      warmupJobs: 4,
      measuredJobs: 24,
      supervisor: {
        activeWorkerEpoch: null,
        workersStarted: 28,
        workersJoined: 28,
        quarantined: false,
      },
    })
    expect(record.samples).toHaveLength(24)
    expect(record.samples.map(sample => sample.job)).toEqual(
      Array.from({ length: 24 }, (_, index) => index),
    )

    const mebibyte = 1024 * 1024
    const first = record.samples[0]
    const last = record.samples.at(-1)!
    expect(slope(record.samples, 'rss')).toBeLessThanOrEqual(2 * mebibyte)
    expect(slope(record.samples, 'heapUsed')).toBeLessThanOrEqual(128 * 1024)
    expect(slope(record.samples, 'external')).toBeLessThanOrEqual(128 * 1024)
    expect(slope(record.samples, 'arrayBuffers')).toBeLessThanOrEqual(64 * 1024)
    expect(last.rss - first.rss).toBeLessThanOrEqual(32 * mebibyte)
    expect(last.heapUsed - first.heapUsed).toBeLessThanOrEqual(4 * mebibyte)
    expect(last.external - first.external).toBeLessThanOrEqual(4 * mebibyte)
    expect(last.arrayBuffers - first.arrayBuffers).toBeLessThanOrEqual(2 * mebibyte)
  }, probeTimeoutMs + 10_000)
})
