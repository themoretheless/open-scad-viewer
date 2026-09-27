export async function run(cancelAt = 80) {
  const worker = new Worker(new URL('./cooperative-bvh.worker.ts', import.meta.url), {type: 'module'})
  const events: unknown[] = []
  let nextId = 0
  const build = (fixture: 'sphere' | 'cube', cancelInside = false) => new Promise<Record<string, unknown>>((resolve, reject) => {
    const id = ++nextId
    let cancelledAt = 0
    const finish = () => {
      clearTimeout(timer)
      worker.removeEventListener('message', message)
      worker.removeEventListener('error', failure)
      worker.removeEventListener('messageerror', failure)
    }
    const failure = (event: Event) => { finish(); reject(Error(`Worker transport failure: ${event.type}`)) }
    const message = (event: MessageEvent) => {
      const data = event.data
      if (data.id !== id) return
      events.push(data)
      if (data.status === 'inside-analysis') {
        if (cancelInside) { cancelledAt = performance.now(); worker.postMessage({type: 'cancel'}) }
        return
      }
      finish()
      if (data.status !== (cancelInside ? 'cancelled' : 'succeeded')) return reject(Error(JSON.stringify(data)))
      if (cancelInside && (!cancelledAt || data.checkpoints < cancelAt)) return reject(Error('Cancellation did not reach the analysis checkpoint'))
      if (!cancelInside && !data.byteParity) return reject(Error('Cooperative result differs from synchronous result'))
      resolve({...data, ...(cancelInside ? {cancelLatencyMs: performance.now() - cancelledAt} : {})})
    }
    const timer = setTimeout(() => {finish(); reject(Error('Analysis scenario timed out'))}, 30_000)
    worker.addEventListener('message', message)
    worker.addEventListener('error', failure)
    worker.addEventListener('messageerror', failure)
    worker.postMessage({type: 'build', id, fixture, pauseAt: cancelInside ? cancelAt : 0})
  })
  try {
    const cancelled = []
    for (let i = 0; i < 4; i++) cancelled.push(await build('sphere', true))
    const recovered = await build('cube')
    if (recovered.volume !== 27 || recovered.triangles !== 12) throw Error('Recovery cube is wrong')
    const completed = await build('sphere')
    return {workerStarts: 1, cancelled, recovered, completed, events,
      scope: 'Real Worker messages cancel during pending Rust analysis steps through the production adapter; same Worker then completes cube and sphere. Render preparation remains synchronous.'}
  } finally {worker.terminate()}
}
