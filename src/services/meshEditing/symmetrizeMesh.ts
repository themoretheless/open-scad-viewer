import { brushPolygonMesh, deformPolygonMesh, sculptPolygonMesh } from '../geometry/polygon'
import type { PolygonMesh } from '../geometry/polygon'
import type { SculptBrush } from '../geometryEditing'
import { finite } from '../meshEditing'
import { flipFaces, joinMeshes } from './parseMeshDocument'

export function symmetrizeMesh(mesh: PolygonMesh, axis: 0 | 1 | 2 = 0): PolygonMesh {
  const mirrored = mesh.positions.slice()
  for (let i = 0; i < mirrored.length / 3; i++) mirrored[i * 3 + axis] *= -1
  const flipped = flipFaces({ positions: mirrored, indices: mesh.indices.slice() })
  return joinMeshes([mesh, flipped])
}

export function brushDisplace(mesh: PolygonMesh, center: [number, number, number], radius: number, displacement: [number, number, number]): PolygonMesh {
  if (!finite(radius) || radius <= 0 || !center.every(finite) || !displacement.every(finite)) throw new Error('Invalid brush.')
  return brushPolygonMesh(mesh, { center, radius, displacement })
}

/** Applies a sculpt stroke (grab/draw/inflate/smooth/flatten/pinch) with falloff and symmetry. */
export function sculptMesh(mesh: PolygonMesh, brush: SculptBrush): PolygonMesh {
  return sculptPolygonMesh(mesh, brush)
}

/** Mean position of `vertices` (all vertices when omitted or empty). */
export function meshCentroid(mesh: PolygonMesh, vertices: readonly number[] = []): [number, number, number] {
  const count = mesh.positions.length / 3
  const picked = vertices.length ? vertices : Array.from({ length: count }, (_, i) => i)
  const center: [number, number, number] = [0, 0, 0]
  for (const v of picked) {
    center[0] += mesh.positions[v * 3]
    center[1] += mesh.positions[v * 3 + 1]
    center[2] += mesh.positions[v * 3 + 2]
  }
  return center.map(c => c / picked.length) as [number, number, number]
}

export function twistMesh(mesh: PolygonMesh, radiansPerUnit: number, origin: [number, number, number] = [0, 0, 0]): PolygonMesh {
  if (!finite(radiansPerUnit) || !origin.every(finite)) throw new Error('Invalid twist.')
  return deformPolygonMesh(mesh, { kind: 'twist', origin, radians_per_unit: radiansPerUnit })
}

export function compactMesh(mesh: PolygonMesh): PolygonMesh {
  const used = new Set(mesh.indices)
  const remap = new Map<number, number>()
  const positions: number[] = []
  for (const id of [...used].sort((a, b) => a - b)) {
    remap.set(id, positions.length / 3)
    positions.push(mesh.positions[id * 3], mesh.positions[id * 3 + 1], mesh.positions[id * 3 + 2])
  }
  return { positions: Float64Array.from(positions), indices: Uint32Array.from(mesh.indices.map(i => remap.get(i)!)) }
}
