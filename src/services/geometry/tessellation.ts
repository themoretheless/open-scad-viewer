/** Compatibility API over the independent Rust polygon library and NURBS adapter. */
import type { NurbsSurface } from '../nurbsSurface'
import { callGeometryRust } from './kernel'
import { inspectPolygonMesh, thickenPolygonMesh, exportPolygonStl, type PolygonBuild } from './polygon'

export type NurbsUV = number[]
export type NurbsTrim = { outer: NurbsUV[]; holes?: NurbsUV[][] }
export type NurbsMesh = PolygonBuild & { faceIds?: number[] }
export type NurbsTessellationOptions = { segmentsU: number; segmentsV: number; trim?: NurbsTrim; maxTriangles?: number }


export function tessellateNurbsSurface(surface: NurbsSurface, options: NurbsTessellationOptions): NurbsMesh {
  return callGeometryRust('surface_tessellate', { surface, options })
}
export function inspectNurbsMesh(positions: number[], indices: number[]): NurbsMesh['report'] {
  return { ...inspectPolygonMesh({ positions, indices }), construction: 'sampled_surface' }
}
export function thickenNurbsMesh(mesh: NurbsMesh, vector: number[]): NurbsMesh {
  const result = thickenPolygonMesh(mesh, vector)
  return { ...result, report: { ...result.report, construction: 'fixed_vector_thickening', sampledDeviationMm: mesh.report.sampledDeviationMm } }
}
export function exportNurbsStl(mesh: NurbsMesh): string { return exportPolygonStl(mesh) }
