import type { MainSolidJob, MainSolidResponse } from './mainSolidProtocol'
import type { StructuralSections } from './structuralSections'

/**
 * Wire packing for the one unbounded CAD response payload: the structural-sections
 * source mesh (up to 900k positions / 300k indices, i.e. roughly 8 MB as numbers).
 * PolygonMesh fields are Float64Array/Uint32Array end to end now, so the worker
 * transfers the freshly created response buffers directly and the host receives a
 * zero-copy move instead of an elementwise structuredClone. Float64Array keeps
 * positions bit-exact; indices are bounded by positions.length / 3, far below 2^32.
 *
 * Only these freshly created response buffers are transferred. Request buffers stay
 * owned by the scene/renderer (transferring them would detach live scene geometry),
 * mirroring the gcodePreviewWorkerTransport precedent and the svgWorkerClient note.
 */
export function prepareMainSolidTransfer(response: MainSolidResponse): { response: MainSolidResponse; transfer: ArrayBuffer[] } {
  if (!response.ok || response.kind !== 'structuralSections') return { response, transfer: [] }
  const result = response.result as StructuralSections
  const mesh = result?.sourceMesh
  if (!mesh || !(mesh.positions instanceof Float64Array) || !(mesh.indices instanceof Uint32Array)) return { response, transfer: [] }
  return { response, transfer: [mesh.positions.buffer as ArrayBuffer, mesh.indices.buffer as ArrayBuffer] }
}

/** Typed arrays survive structuredClone intact, so protocol validation sees them
 * directly; this stays as the seam for any future packing. */
export function decodeMainSolidResult(kind: MainSolidJob['kind'], result: unknown): unknown {
  return result
}
