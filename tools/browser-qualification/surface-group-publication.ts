import {withSelectionSurfaces} from '../../src/services/meshSurfaceGroups'
import {GEOMETRY_WORKER_PROTOCOL_VERSION, isGeometryWorkerEvent, type GeometryBuildSuccess} from '../../src/services/geometryWorkerProtocol'
import {sha256Hex} from '../../src/core/sha256'

export async function run(workerUrl: string) {
  const workers = [new Worker(workerUrl, {type: 'module'}), new Worker(workerUrl, {type: 'module'})]
  let id = 0
  const build = (mode: number, source: string): Promise<{ms: number; selectionMs: number; result: GeometryBuildSuccess}> => {
    const jobId = ++id
    const sourceSha256 = sha256Hex(source)
    return new Promise((resolve, reject) => {
      const worker = workers[mode]
      const finish = (error?: Error) => {
        clearTimeout(timer)
        worker.removeEventListener('message', message)
        worker.removeEventListener('error', failed)
        if (error) reject(error)
      }
      const failed = (event: ErrorEvent) => finish(Error(event.message))
      const message = (event: MessageEvent) => {
        const result = event.data
        if (result.jobId !== jobId) return
        if (!isGeometryWorkerEvent(result)) return finish(Error('Invalid worker packet'))
        if (result.status === 'failed' || result.status === 'cancelled' || result.status === 'stale') return finish(Error(JSON.stringify(result)))
        if (result.status !== 'succeeded') return
        const selectionStart = performance.now()
        result.meshes = result.meshes.map(withSelectionSurfaces)
        const end = performance.now()
        finish()
        resolve({ms: end - start, selectionMs: end - selectionStart, result})
      }
      worker.addEventListener('message', message)
      worker.addEventListener('error', failed)
      const timer = setTimeout(() => finish(Error('Build timeout')), 30000)
      const start = performance.now()
      worker.postMessage({protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION, type: 'build',
        documentRevision: jobId, jobId, quality: 'full', source, sourceSha256,
        ...(mode ? {selectionSurfaces: true} : {})})
    })
  }
  try {
    const cold = []
    for (let mode = 0; mode < 2; mode++) cold.push((await build(mode, 'cube(2);')).ms)
    const reports = []
    for (const fixture of ['cube', 'sphere']) for (const cache of ['miss', 'hit']) {
      const samples: number[][] = [[], []], selection: number[][] = [[], []]
      let triangles = 0
      for (let i = 0; i < 12; i++) {
        const size = cache === 'hit' ? 10 : 20 + i / 8
        const source = fixture === 'cube' ? `cube(${size});` : `sphere(r=${size},$fn=128);`
        const pair = []
        for (const mode of i % 2 ? [1, 0] : [0, 1]) pair[mode] = await build(mode, source)
        const [a, b] = pair.map(p => p.result)
        if (a.meshes.length !== b.meshes.length) throw Error('Mesh count mismatch')
        triangles = a.meshes.reduce((n,m) => n + m.indices.length / 3, 0)
        for (let j = 0; j < a.meshes.length; j++) {
          const x = a.meshes[j].faceIds, y = b.meshes[j].faceIds
          if (x.length !== y.length || x.some((v,k) => v !== y[k])) throw Error('Surface ID mismatch')
          if (!b.meshes[j].faceIdsInferred) throw Error('Worker omitted inferred metadata')
        }
        if (i >= 3) for (const mode of [0, 1]) {
          samples[mode].push(pair[mode].ms)
          selection[mode].push(pair[mode].selectionMs)
        }
      }
      const median = (xs: number[]) => [...xs].sort((a,b) => a-b)[Math.floor(xs.length / 2)]
      reports.push({fixture, cache, triangles, totalP50: samples.map(median), mainSelectionP50: selection.map(median), samples, selection})
    }
    return {cold, reports, scope: 'worker build + transfer + packet validation + selection; excludes GPU upload and paint', modes: ['host TS', 'worker Rust']}
  } finally { workers.forEach(w => w.terminate()) }
}
