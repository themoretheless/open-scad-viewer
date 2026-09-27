import {BuildCoordinator, type BuildCoordinatorOptions, type PublishedGeometryBuild} from '../../src/services/buildCoordinator'
import {sha256Hex} from '../../src/core/sha256'

const SCENARIO_TIMEOUT_MS = 30_000
const SILENCE_TIMEOUT_MS = 5_000
const MULTI_STATEMENT_SOURCE = Array.from({length: 256}, (_, i) => `translate([${i * 3},0,0]) sphere(r=1,$fn=48);`).join('\n')
const SINGLE_STATEMENT_SOURCE = 'union() { for (i=[0:255]) translate([i*3,0,0]) sphere(r=1,$fn=48); }'
const WEDGE_SOURCE = '// qualification-wedge\ncube(7);'

type Publication = {status: string; revision: number; jobId: number; sourceSha256: string; volume?: number; code?: string; ms: number}
type WorkerEvent = {generation: number; status: string; phase: string; revision: number; jobId: number; ms: number}

function check(condition: unknown, message: string): void {
  if (!condition) throw Error(message)
}

/** Real production worker with an explicitly injected, non-kernel event-loop wedge. */
function faultWorkerUrl(workerUrl: string): string {
  const absoluteWorkerUrl = new URL(workerUrl, location.href).href
  return URL.createObjectURL(new Blob([`
    let ready = false;
    const queued = [];
    self.addEventListener('message', event => {
      if (!ready) {
        event.stopImmediatePropagation();
        queued.push(event.data);
        return;
      }
      if (event.data?.type === 'build' && event.data.source.includes('// qualification-wedge')) {
        event.stopImmediatePropagation();
        while (true) {} // Deliberate qualification fault; terminated by the host.
      }
    });
    await import(${JSON.stringify(absoluteWorkerUrl)});
    ready = true;
    for (const data of queued) self.dispatchEvent(new MessageEvent('message', {data}));
    queued.length = 0;
  `], {type: 'text/javascript'}))
}

function createRig(workerUrl: string, options: Partial<BuildCoordinatorOptions> = {}, faultGenerations = 0) {
  const start = performance.now()
  const deadline = start + SCENARIO_TIMEOUT_MS
  const events: WorkerEvent[] = []
  const publications: Publication[] = []
  const restarts: unknown[] = []
  const workers: Worker[] = []
  const faultUrl = faultGenerations ? faultWorkerUrl(workerUrl) : null
  let disposed = false
  const coordinator = new BuildCoordinator({
    snapshotEvents: false,
    selectionSurfaces: true,
    supersedeGraceMs: 300,
    workerSilenceTimeoutMs: SILENCE_TIMEOUT_MS,
    ...options,
    workerFactory: () => {
      const generation = workers.length + 1
      const worker = new Worker(faultUrl && generation <= faultGenerations ? faultUrl : workerUrl, {type: 'module'})
      workers.push(worker)
      worker.addEventListener('message', event => {
        const data = event.data
        events.push({generation, status: data.status, phase: data.phase, revision: data.documentRevision, jobId: data.jobId, ms: performance.now() - start})
      })
      return worker
    },
    onWorkerRestart: () => restarts.push({ms: performance.now() - start}),
    onPublish: (event: PublishedGeometryBuild) => publications.push({
      status: event.status, revision: event.documentRevision, jobId: event.jobId,
      sourceSha256: event.sourceSha256,
      ...(event.status === 'succeeded' ? {volume: event.volume} : {code: event.error.code}),
      ms: performance.now() - start,
    }),
  })
  const waitFor = async (predicate: () => unknown, description: string) => {
    while (!predicate()) {
      check(!disposed, `Disposed while waiting for ${description}`)
      check(coordinator.state.status !== 'failed', `Build failed while waiting for ${description}: ${JSON.stringify({state: coordinator.state, events, publications})}`)
      check(performance.now() < deadline, `Scenario timed out waiting for ${description}: ${JSON.stringify({state: coordinator.state, events, publications})}`)
      await new Promise(resolve => setTimeout(resolve, 10))
    }
  }
  const submit = (revision: number, source: string) => coordinator.requestBuild({documentRevision: revision, source, quality: 'full'})
  const cube = async (revision: number, size: number) => {
    const source = `cube(${size});`
    submit(revision, source)
    await waitFor(() => publications.some(event => event.revision === revision), `cube revision ${revision}`)
    const outcome = publications.find(event => event.revision === revision)!
    check(outcome.status === 'succeeded', `Cube failed: ${JSON.stringify(outcome)}`)
    check(Math.abs(outcome.volume! - size ** 3) < 1e-6, `Wrong cube volume: ${JSON.stringify(outcome)}`)
    check(outcome.sourceSha256 === sha256Hex(source), 'Cube source attestation mismatch')
    check(coordinator.state.status === 'ready', 'Cube did not restore ready state')
    return outcome
  }
  const heavyProgress = (revision: number) => events.filter(event => event.revision === revision && event.status === 'progress' && event.phase === 'compiling')
  const waitForHeavy = async (revision: number) => {
    await waitFor(() => heavyProgress(revision).length > 0, 'heavy multi-statement compilation')
    await new Promise(resolve => setTimeout(resolve, 50))
    check(!events.some(event => event.revision === revision && ['succeeded', 'failed', 'cancelled', 'stale'].includes(event.status)), `Heavy multi-statement build finished before cancellation: ${JSON.stringify(events)}`)
  }
  const report = (name: string, assertions: string[], faultInjection = 'none') => ({
    name, passed: true, elapsedMs: performance.now() - start, faultInjection, assertions,
    diagnostics: coordinator.diagnostics, workerRestarts: restarts, events, publications,
  })
  const dispose = () => {
    coordinator.dispose()
    disposed = true
    workers.forEach(worker => worker.terminate())
    if (faultUrl) URL.revokeObjectURL(faultUrl)
  }
  return {coordinator, events, publications, restarts, cube, submit, waitFor, waitForHeavy, heavyProgress, report, dispose}
}

export async function run(workerUrl: string) {
  const reports = []
  {
    const rig = createRig(workerUrl)
    try {
      await rig.cube(1, 2)
      rig.submit(2, MULTI_STATEMENT_SOURCE)
      await rig.waitForHeavy(2)
      check(!rig.publications.some(event => event.revision === 2), 'Heavy build finished before supersession')
      await rig.cube(3, 5)
      check(!rig.publications.some(event => event.revision === 2), 'Superseded geometry was published')
      check(rig.coordinator.diagnostics.workerStarts === 1, `Cooperative supersession lost the warm worker: ${JSON.stringify(rig.report('warm-multi-statement-supersession', []))}`)
      check(rig.coordinator.diagnostics.hardRestarts === 0, `Cooperative supersession needed hard preemption: ${JSON.stringify(rig.report('warm-multi-statement-supersession', []))}`)
      check(rig.events.some(event => event.revision === 2 && ['cancelled', 'stale'].includes(event.status)), 'Worker did not acknowledge obsolete work')
      reports.push(rig.report('warm-multi-statement-supersession', ['real multi-statement build remained active for 50ms', 'obsolete build acknowledged without publication within the production 300ms grace', 'same warm worker retained', 'replacement cube volume = 125']))
    } finally { rig.dispose() }
  }
  {
    const rig = createRig(workerUrl, {supersedeGraceMs: 0})
    try {
      await rig.cube(1, 2)
      rig.submit(2, SINGLE_STATEMENT_SOURCE)
      await rig.waitFor(() => rig.heavyProgress(2).length > 0, 'single-statement compilation')
      await new Promise(resolve => setTimeout(resolve, 100))
      check(!rig.events.some(event => event.revision === 2 && ['succeeded', 'failed', 'cancelled', 'stale'].includes(event.status)), `Heavy single statement finished before preemption: ${JSON.stringify(rig.events)}`)
      await rig.cube(3, 6)
      check(!rig.publications.some(event => event.revision === 2), 'Hard-preempted geometry was published')
      check(rig.coordinator.diagnostics.workerStarts === 2, 'Hard preemption did not create one replacement worker')
      check(rig.coordinator.diagnostics.hardRestarts === 1, 'Expected exactly one hard preemption')
      reports.push(rig.report('busy-worker-hard-preemption', ['real single-statement build remained active for 100ms', 'worker replaced once', 'obsolete build never published', 'replacement cube volume = 216']))
    } finally { rig.dispose() }
  }
  {
    const rig = createRig(workerUrl)
    try {
      await rig.cube(1, 2)
      rig.submit(2, MULTI_STATEMENT_SOURCE)
      await rig.waitForHeavy(2)
      rig.coordinator.cancel('user')
      check(rig.coordinator.state.status === 'cancelled', 'User cancel did not reach cancelled state')
      await rig.cube(3, 3)
      check(!rig.publications.some(event => event.revision === 2), 'Cancelled build was published')
      check(rig.coordinator.diagnostics.workerStarts === 2, 'User cancel did not replace the worker for the next build')
      reports.push(rig.report('user-cancel-and-rebuild', ['user cancel reached cancelled state', 'cancelled result never published', 'subsequent cube volume = 27']))
    } finally { rig.dispose() }
  }
  {
    const rig = createRig(workerUrl, {}, 1)
    try {
      await rig.cube(1, 2)
      const jobId = rig.submit(2, WEDGE_SOURCE)
      await rig.waitFor(() => rig.publications.some(event => event.revision === 2), 'one silence timeout and production-worker retry')
      const outcome = rig.publications.find(event => event.revision === 2)!
      check(outcome.status === 'succeeded' && Math.abs(outcome.volume! - 343) < 1e-6, `Timeout retry failed: ${JSON.stringify(outcome)}`)
      check(outcome.jobId === jobId && outcome.sourceSha256 === sha256Hex(WEDGE_SOURCE), 'Retry changed job/source identity')
      check(rig.coordinator.diagnostics.workerStarts === 2 && rig.restarts.length === 1, 'Expected one timeout restart')
      check(rig.publications.filter(event => event.revision === 2).length === 1, 'Retry published more than once')
      reports.push(rig.report('silence-timeout-retry', ['one retry on a new production worker', 'job/revision/source identity retained', 'single success volume = 343'], 'first generation blocks its event loop on a marked request after a real warmup cube'))
    } finally { rig.dispose() }
  }
  {
    const rig = createRig(workerUrl, {}, 2)
    try {
      await rig.cube(1, 2)
      rig.submit(2, WEDGE_SOURCE)
      await rig.waitFor(() => rig.publications.some(event => event.revision === 2), 'terminal timeout after the one allowed retry')
      const outcome = rig.publications.find(event => event.revision === 2)!
      check(outcome.status === 'failed' && outcome.code === 'WORKER_TIMEOUT', `Wrong terminal failure: ${JSON.stringify(outcome)}`)
      check(rig.coordinator.state.status === 'failed', 'Repeated timeout did not reach failed state')
      check(rig.coordinator.diagnostics.workerStarts === 2, 'Repeated timeout exceeded the retry budget')
      await rig.cube(3, 4)
      check(rig.publications.filter(event => event.revision === 2).length === 1, 'Timed-out revision published more than once')
      check(rig.coordinator.diagnostics.workerStarts === 3, 'New revision did not get a fresh worker')
      reports.push(rig.report('terminal-timeout-and-new-revision', ['retry budget stops at one', 'single WORKER_TIMEOUT publication', 'new revision recovers with cube volume = 64'], 'first two generations block their event loops on marked requests'))
    } finally { rig.dispose() }
  }
  {
    const rig = createRig(workerUrl)
    try {
      await rig.cube(1, 2)
      rig.submit(2, MULTI_STATEMENT_SOURCE)
      await rig.waitForHeavy(2)
      const publishedBeforeDispose = rig.publications.length
      rig.coordinator.dispose()
      await new Promise(resolve => setTimeout(resolve, 150))
      check(rig.coordinator.state.status === 'disposed', 'Dispose did not reach disposed state')
      check(rig.publications.length === publishedBeforeDispose, 'Result published after disposal')
      check(rig.coordinator.state.activeJobIds.length === 0, 'Dispose retained active jobs')
      reports.push(rig.report('dispose-during-build', ['active jobs cleared', 'no publication after disposal']))
    } finally { rig.dispose() }
  }
  return {
    scope: 'Real Chromium Worker transport, production geometry worker and BuildCoordinator. Cooperative cancellation is between statements; no claim of interruption inside a synchronous WASM kernel call.',
    scenarioTimeoutMs: SCENARIO_TIMEOUT_MS,
    faultSilenceTimeoutMs: SILENCE_TIMEOUT_MS,
    fixtures: {multiStatement: sha256Hex(MULTI_STATEMENT_SOURCE), singleStatement: sha256Hex(SINGLE_STATEMENT_SOURCE), faultRequest: sha256Hex(WEDGE_SOURCE)},
    reports,
  }
}
