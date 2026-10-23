import type {MeshData} from '../core/mesh'
import {surfaceGroupsInKernel} from './geometry/meshAnalysis'
import {createSelectionSurfacePublisher} from './selectionSurfacePublisher'
/** Connected smooth patches inferred by the native topology kernel. */
export function inferSurfaceIds(vertices:Float32Array,indices:Uint32Array,angleDegrees=30):Uint32Array {
 if(vertices.byteLength > 16 * 1024 * 1024) {
  // Pack only referenced records to fit the bounded WASM transfer. Topology stays native.
  if(vertices.length%6 || indices.length%3) throw new Error('Invalid surface grouping input/budget')
  for(let i=0;i<vertices.length;i+=6) if(!Number.isFinite(vertices[i])||!Number.isFinite(vertices[i+1])||!Number.isFinite(vertices[i+2])) throw new Error('Nonfinite mesh position')
  const remap=new Map<number,number>(), packed:number[]=[], mapped=new Uint32Array(indices.length)
  for(let i=0;i<indices.length;i++) {
   const source=indices[i];if(source>=vertices.length/6)throw new Error('Invalid triangle index')
   let id=remap.get(source)
   if(id===undefined){id=remap.size;remap.set(source,id);for(let k=0;k<6;k++)packed.push(vertices[source*6+k])}
   mapped[i]=id
  }
  return surfaceGroupsInKernel(new Float32Array(packed),mapped,6,angleDegrees)
 }
 return surfaceGroupsInKernel(vertices,indices,6,angleDegrees)
}
/** Preserve authored face IDs exactly; only legacy meshes use inferred patches. */
export function withSelectionSurfaces(mesh:MeshData):MeshData {
 return publishSelectionSurfaces(mesh)
}

const publishSelectionSurfaces = createSelectionSurfacePublisher(
 (vertices, indices) => surfaceGroupsInKernel(vertices, indices, 6),
 { cloneResult: false },
)

export interface SurfaceGroupAreaSummary {
  surfaceId: number
  triangleCount: number
  areaMm2: number
  averageNormal: [number, number, number]
}

/**
 * Groups triangles by `surfaceId` and computes total surface area (`areaMm2`)
 * and the area-weighted average unit normal per group in a single pass.
 */
export function computeSurfaceGroupAreaSummary(
  mesh: { positions: ArrayLike<number>; indices: ArrayLike<number> },
  surfaceIds: ArrayLike<number>,
): SurfaceGroupAreaSummary[] {
  const { positions, indices } = mesh
  const triCount = Math.min(Math.floor(indices.length / 3), surfaceIds.length)
  if (triCount <= 0 || positions.length < 9) return []

  const accumulators = new Map<
    number,
    { triangleCount: number; areaMm2: number; nx: number; ny: number; nz: number }
  >()

  for (let t = 0; t < triCount; t++) {
    const surfaceId = surfaceIds[t]!
    const base = t * 3
    const i0 = indices[base]! * 3
    const i1 = indices[base + 1]! * 3
    const i2 = indices[base + 2]! * 3
    if (i0 + 2 >= positions.length || i1 + 2 >= positions.length || i2 + 2 >= positions.length) {
      continue
    }

    const ax = positions[i0]!
    const ay = positions[i0 + 1]!
    const az = positions[i0 + 2]!
    const bx = positions[i1]! - ax
    const by = positions[i1 + 1]! - ay
    const bz = positions[i1 + 2]! - az
    const cx = positions[i2]! - ax
    const cy = positions[i2 + 1]! - ay
    const cz = positions[i2 + 2]! - az

    // Cross product (b - a) x (c - a) has magnitude = 2 * triangleArea
    const crossX = by * cz - bz * cy
    const crossY = bz * cx - bx * cz
    const crossZ = bx * cy - by * cx
    const crossLen = Math.hypot(crossX, crossY, crossZ)
    if (!Number.isFinite(crossLen)) continue

    const triArea = 0.5 * crossLen
    let entry = accumulators.get(surfaceId)
    if (!entry) {
      entry = { triangleCount: 0, areaMm2: 0, nx: 0, ny: 0, nz: 0 }
      accumulators.set(surfaceId, entry)
    }
    entry.triangleCount += 1
    entry.areaMm2 += triArea
    // Area-weighted normal contribution is 0.5 * cross = triArea * unitNormal
    entry.nx += 0.5 * crossX
    entry.ny += 0.5 * crossY
    entry.nz += 0.5 * crossZ
  }

  const summaries: SurfaceGroupAreaSummary[] = []
  for (const [surfaceId, acc] of accumulators.entries()) {
    const normLen = Math.hypot(acc.nx, acc.ny, acc.nz)
    const averageNormal: [number, number, number] =
      normLen > 1e-12
        ? [acc.nx / normLen, acc.ny / normLen, acc.nz / normLen]
        : [0, 0, 0]
    summaries.push({
      surfaceId,
      triangleCount: acc.triangleCount,
      areaMm2: acc.areaMm2,
      averageNormal,
    })
  }

  summaries.sort((a, b) => a.surfaceId - b.surfaceId)
  return summaries
}
