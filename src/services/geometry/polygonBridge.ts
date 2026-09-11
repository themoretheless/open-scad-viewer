/** Explicit interchange between the two Rust libraries; no inferred smooth fit. */
import type { NurbsCurve } from '../nurbsCurve'
import type { NurbsSurface } from '../nurbsSurface'
import type { PolygonBuild, PolygonMesh } from './polygon'
import { callGeometryRust } from './kernel'
import type { NurbsTessellationOptions } from './tessellation'
export type SurfaceMeshingOptions = NurbsTessellationOptions
export function nurbsToPolygonMesh(surface: NurbsSurface, options: SurfaceMeshingOptions): PolygonBuild {
  return callGeometryRust('surface_tessellate', { surface, options })
}
/** Each closed boundary becomes a clamped degree-one NURBS curve. */
export function polygonBoundaryNurbsCurves(mesh: PolygonMesh): NurbsCurve[] {
  return callGeometryRust('mesh_boundary_curves', { mesh })
}
