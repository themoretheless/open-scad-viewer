import {CadGeometryKernel} from '../../src/services/cadGeometryKernel'
import {analyzeSolidCooperativelyInKernel, analyzeSolidInKernel, type KernelSolidAnalysis} from '../../src/services/geometry/meshAnalysis'

let active = false
let cancelled = false
const post = (value: unknown) => self.postMessage(value, {transfer: []})
const bytes = (view: ArrayBufferView) => new Uint8Array(view.buffer, view.byteOffset, view.byteLength)
function same(actual: KernelSolidAnalysis, expected: KernelSolidAnalysis) {
  const pairs = [
    [actual.mesh.vertices, expected.mesh.vertices], [actual.mesh.indices, expected.mesh.indices],
    [actual.mesh.mergeFrom, expected.mesh.mergeFrom], [actual.mesh.mergeTo, expected.mesh.mergeTo],
    [actual.mesh.faceIds, expected.mesh.faceIds], [actual.bvh.bounds, expected.bvh.bounds],
    [actual.bvh.nodes, expected.bvh.nodes], [actual.bvh.triangles, expected.bvh.triangles],
    [actual.semanticEdges.indices, expected.semanticEdges.indices],
  ]
  return pairs.every(([a,b]) => a.byteLength === b.byteLength && bytes(a).every((v,i) => v === bytes(b)[i]))
    && JSON.stringify(actual.semanticEdges.diagnostics) === JSON.stringify(expected.semanticEdges.diagnostics)
}
self.addEventListener('message', event => {
  if (event.data?.type === 'cancel') { cancelled = true; return }
  if (event.data?.type !== 'build' || active) return
  active = true
  cancelled = false
  const {id, fixture, pauseAt} = event.data as {id: number; fixture: 'sphere' | 'cube'; pauseAt: number}
  void (async () => {
    const session = await new CadGeometryKernel().openSession()
    let checkpoints = 0
    const start = performance.now()
    try {
      const {CadSolid} = session.module
      const solid = fixture === 'sphere' ? CadSolid.sphere(3, 128) : CadSolid.cube(3)
      const expected = analyzeSolidInKernel(solid.handle)
      const actual = await analyzeSolidCooperativelyInKernel(solid.handle, async () => {
        checkpoints++
        // This checkpoint occurs after many pending Rust analysis steps, before
        // publication. Give a real cancel message a macrotask.
        if (checkpoints === pauseAt) post({id, status: 'inside-analysis', checkpoints})
        if (checkpoints % 8 === 0 || checkpoints === pauseAt) await new Promise(resolve => setTimeout(resolve, 0))
        if (cancelled) throw new DOMException('Build cancelled', 'AbortError')
      })
      post({id, status: 'succeeded', checkpoints, elapsedMs: performance.now() - start,
        byteParity: same(actual, expected), triangles: actual.mesh.indices.length / 3,
        volume: solid.volume(), nodeCount: actual.bvh.nodeCount})
    } catch (error) {
      post({id, status: error instanceof DOMException && error.name === 'AbortError' ? 'cancelled' : 'failed',
        checkpoints, elapsedMs: performance.now() - start, error: String(error)})
    } finally { session.dispose(); active = false }
  })().catch(error => { active = false; post({id, status: 'failed', error: String(error)}) })
})
