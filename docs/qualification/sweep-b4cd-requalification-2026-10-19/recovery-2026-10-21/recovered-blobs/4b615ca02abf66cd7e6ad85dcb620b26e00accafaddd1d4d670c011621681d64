import type { DirectBody } from './directModeling'
import { callGeometryRust } from './geometry/kernel'
import { normalizePolygonMesh, type PolygonMesh } from './geometry/polygon'

export function extendedShell(
  body: DirectBody,
  openings: number[],
  thickness: number,
  step?: number,
  adaptive = false,
): DirectBody {
  if (body.brep) {
    throw new Error('Mesh shell cannot be claimed as analytic-shell for B-rep bodies; refuse sampled/mesh fallback')
  }
  const mesh = normalizePolygonMesh(
    callGeometryRust<PolygonMesh>('cad_extended_shell', {
      mesh: body.mesh,
      openings,
      thickness,
      step: step ?? 0,
      adaptive,
    }),
  )
  return { ...body, mesh: { positions: mesh.positions, indices: mesh.indices } }
}

export function extendedBevel(
  body: DirectBody,
  edgeIndex: number,
  size: number,
  kind: 'fillet' | 'chamfer',
): DirectBody {
  if (body.brep) {
    throw new Error(`Mesh ${kind} cannot be claimed as analytic-${kind} for B-rep bodies; refuse localMeshBevel fallback`)
  }
  const mesh = normalizePolygonMesh(
    callGeometryRust<PolygonMesh>('cad_extended_bevel', {
      mesh: body.mesh,
      edge: edgeIndex,
      size,
      kind,
    }),
  )
  return { ...body, mesh: { positions: mesh.positions, indices: mesh.indices } }
}
