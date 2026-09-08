/** Explicit interchange between the two Rust libraries; no inferred smooth fit. */
import type { NurbsCurve } from './nurbsCurve'
import type { NurbsSurface } from './nurbsSurface'
import type { PolygonBuild, PolygonMesh } from './polygonKernel'
import { callGeometryRust } from './geometryRustKernel'
export interface SurfaceMeshingOptions {
  segmentsU: number
  segmentsV: number
  trim?: { outer: number[][]; holes?: number[][][] }
  maxTriangles?: number
}
export function nurbsToPolygonMesh(surface: NurbsSurface, options: SurfaceMeshingOptions): PolygonBuild {
  return callGeometryRust('surface_tessellate', { surface, options })
}
/** Each closed boundary becomes a clamped degree-one NURBS curve. */
export function polygonBoundaryNurbsCurves(mesh: PolygonMesh): NurbsCurve[] {
  return callGeometryRust('mesh_boundary_curves', { mesh })
}
