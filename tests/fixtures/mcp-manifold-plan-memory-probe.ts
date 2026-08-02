import { McpManifoldPlanQualificationSupervisor } from '../../src/mcp/manifoldPlanQualificationSupervisor'

const WARMUP_JOBS = 4
const MEASURED_JOBS = 24

interface MemorySample {
  readonly job: number
  readonly rss: number
  readonly heapUsed: number
  readonly external: number
  readonly arrayBuffers: number
}

async function collect(job: number): Promise<MemorySample> {
  const gc = (globalThis as typeof globalThis & { gc?: () => void }).gc
  if (gc === undefined) throw new Error('Memory probe requires --expose-gc')
  for (let pass = 0; pass < 3; pass++) {
    gc()
    await new Promise<void>(resolve => setImmediate(resolve))
  }
  const usage = process.memoryUsage()
  return {
    job,
    rss: usage.rss,
    heapUsed: usage.heapUsed,
    external: usage.external,
    arrayBuffers: usage.arrayBuffers,
  }
}

const supervisor = new McpManifoldPlanQualificationSupervisor({
  startupTimeoutMs: 10_000,
  deadlineMs: 10_000,
  cancellationGraceMs: 0,
  joinTimeoutMs: 2_000,
})

for (let job = 0; job < WARMUP_JOBS; job++) {
  await supervisor.evaluate('cube(1);')
}

const samples: MemorySample[] = []
for (let job = 0; job < MEASURED_JOBS; job++) {
  await supervisor.evaluate('cube(1);')
  samples.push(await collect(job))
}

process.stdout.write(`${JSON.stringify({
  schema: 'mcp-manifold-plan-memory-probe',
  version: 1,
  warmupJobs: WARMUP_JOBS,
  measuredJobs: MEASURED_JOBS,
  supervisor: supervisor.snapshot(),
  samples,
})}\n`)
