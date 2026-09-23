import type { MainSolidJob, MainSolidResponse } from './mainSolidProtocol'
import type { StructuralSections } from './structuralSections'

/**
 * Wire packing for the one unbounded CAD response payload: the structural-sections
 * source mesh (up to 900k positions / 300k indices, i.e. roughly 8 MB as numbers).
 * The worker packs the freshly decoded plain arrays into typed arrays and transfers
 * their buffers, so the host receives them as a zero-copy move instead of an
 * elementwise structuredClone. Float64Array keeps positions bit-exact; indices are
 * bounded by positions.length / 3, far below 2^32.
 *
 * Only these freshly created response buffers are transferred. Request buffers stay
 * owned by the scene/renderer (transferring them would detach live scene geometry),
 * mirroring the gcodePreviewWorkerTransport precedent and the svgWorkerClient note.
 */
export function prepareMainSolidTransfer(response: MainSolidResponse): { response: MainSolidResponse; transfer: ArrayBuffer[] } {
  if (!response.ok || response.kind !== 'structuralSections') return { response, transfer: [] }
  const result = response.result as StructuralSections
  const mesh = result?.sourceMesh
  if (!mesh || !Array.isArray(mesh.positions) || !Array.isArray(mesh.indices)) return { response, transfer: [] }
  const positions = Float64Array.from(mesh.positions)
  const indices = Uint32Array.from(mesh.indices)
  return {
    response: {
      ...response,
      result: { ...result, sourceMesh: { ...mesh,
        positions: positions as unknown as number[], indices: indices as unknown as number[] } },
    },
    transfer: [positions.buffer, indices.buffer],
  }
}

/** Restore the plain-array contract before protocol validation; identity fast path otherwise. */
export function decodeMainSolidResult(kind: MainSolidJob['kind'], result: unknown): unknown {
  if (kind !== 'structuralSections' || !result || typeof result !== 'object') return result
  const mesh = (result as StructuralSections).sourceMesh
  if (!mesh || typeof mesh !== 'object') return result
  const { positions, indices } = mesh as { positions: unknown; indices: unknown }
  const packedPositions = positions instanceof Float64Array
  const packedIndices = indices instanceof Uint32Array
  if (!packedPositions && !packedIndices) return result
  return {
    ...(result as object),
    sourceMesh: {
      ...mesh,
      positions: packedPositions ? Array.from(positions) : positions,
      indices: packedIndices ? Array.from(indices) : indices,
    },
  }
}
