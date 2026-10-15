import type {PolygonMesh} from './geometry/polygon'
import type {DisplayMesh} from './solidDisplayCache'
import type { MainSolidJob, MainSolidResponse } from './mainSolidProtocol'
import type { StructuralSections } from './structuralSections'
import type { DirectDocument } from './directModeling'

/**
 * Wire packing for restored documents and the structural-sections source mesh
 * (up to 900k positions / 300k indices, i.e. roughly 8 MB as numbers).
 * PolygonMesh fields are Float64Array/Uint32Array end to end now, so the worker
 * transfers the freshly created response buffers directly and the host receives a
 * zero-copy move instead of an elementwise structuredClone. Float64Array keeps
 * positions bit-exact; indices are bounded by positions.length / 3, far below 2^32.
 *
 * Only these freshly created response buffers are transferred. Request buffers stay
 * owned by the scene/renderer (transferring them would detach live scene geometry),
 * mirroring the gcodePreviewWorkerTransport precedent and the svgWorkerClient note.
 */
function pushTransferView(transfer: ArrayBuffer[], view: unknown): void {
  if (view && ArrayBuffer.isView(view)) {
    const buf = view.buffer
    if (buf instanceof ArrayBuffer && !transfer.includes(buf)) {
      transfer.push(buf)
    }
  }
}

export function prepareMainSolidTransfer(response: MainSolidResponse): { response: MainSolidResponse; transfer: ArrayBuffer[] } {
  if(response.ok && (response.kind==='displayMesh'||response.kind==='surfaceMesh')) {
    const mesh=response.kind==='surfaceMesh'?response.result as PolygonMesh:(response.result as DisplayMesh).mesh
    const transfer: ArrayBuffer[] = []
    pushTransferView(transfer, mesh.positions)
    pushTransferView(transfer, mesh.indices)
    pushTransferView(transfer, mesh.uv)
    return {response,transfer}
  }
  if(response.ok && (response.kind==='restoreDocument'||response.kind==='rushGraphImport'||response.kind==='primitive')) {
    // These jobs parse the complete replacement, so parsed mesh buffers belong
    // exclusively to the response. Deduplicate buffers in case views share backing storage.
    const bodies = (response.result as DirectDocument).bodies
    if (bodies.length <= 8) {
      const transfer: ArrayBuffer[] = []
      for (let i = 0; i < bodies.length; i++) {
        const mesh = bodies[i]!.mesh
        pushTransferView(transfer, mesh.positions)
        pushTransferView(transfer, mesh.indices)
        pushTransferView(transfer, mesh.uv)
      }
      return { response, transfer }
    }
    const buffers=new Set<ArrayBuffer>()
    for(const body of bodies) {
      const mesh = body.mesh
      if(mesh.positions && ArrayBuffer.isView(mesh.positions) && mesh.positions.buffer instanceof ArrayBuffer)buffers.add(mesh.positions.buffer)
      if(mesh.indices && ArrayBuffer.isView(mesh.indices) && mesh.indices.buffer instanceof ArrayBuffer)buffers.add(mesh.indices.buffer)
      if(mesh.uv && ArrayBuffer.isView(mesh.uv) && mesh.uv.buffer instanceof ArrayBuffer)buffers.add(mesh.uv.buffer)
    }
    return {response,transfer:[...buffers]}
  }
  if (!response.ok || response.kind !== 'structuralSections') return { response, transfer: [] }
  const result = response.result as StructuralSections
  const mesh = result?.sourceMesh
  if (!mesh || !(mesh.positions instanceof Float64Array) || !(mesh.indices instanceof Uint32Array)) return { response, transfer: [] }
  const posBuf = mesh.positions.buffer as ArrayBuffer
  const idxBuf = mesh.indices.buffer as ArrayBuffer
  return { response, transfer: posBuf === idxBuf ? [posBuf] : [posBuf, idxBuf] }
}

/** Typed arrays survive structuredClone intact, so protocol validation sees them
 * directly; this stays as the seam for any future packing. */
export function decodeMainSolidResult(kind: MainSolidJob['kind'], result: unknown): unknown {
  return result
}
