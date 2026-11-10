import {callGeometryRust} from './geometry/kernel'
import {normalizePolygonMesh,type PolygonMesh} from './geometry/polygon'
import type {NurbsBrep} from './geometry/brep'
import type {DisplayMesh} from './solidDisplayCache'
/** Numerical display preparation stays in Rust; the UI owns cache keys and lifetime. */
export function prepareSolidDisplay(mesh:PolygonMesh,brep?:NurbsBrep,segments=12):DisplayMesh {
 const result=callGeometryRust<DisplayMesh>('cad_display_mesh',{mesh,...(brep?{brep}:{}),segments,maxTriangles:4000})
 normalizePolygonMesh(result.mesh)
 return result
}
