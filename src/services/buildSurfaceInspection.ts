import type {PolygonMesh} from './geometry/polygon'
import {callGeometryRust} from './geometry/kernel'

export interface BuildSurfaceOptions {
  buildDirection: [number,number,number]
  coneDegrees: number
  planeOffsetMm: number
  planeToleranceMm: number
}
export interface BuildSurfaceReport {
  modelKind: 'signed-triangle-build-surfaces-v1'
  totalAreaMm2: number
  downwardAreaMm2: number
  downwardTriangles: number
  contactAreaMm2: number
  belowPlaneTriangles: number
}

/** Whole-triangle geometric classification, not support prediction or print certification. */
export function inspectBuildSurfaces(mesh: PolygonMesh, options: BuildSurfaceOptions): BuildSurfaceReport {
  return callGeometryRust('mesh_build_surfaces', {...options, mesh:{positions:mesh.positions,indices:mesh.indices}})
}
