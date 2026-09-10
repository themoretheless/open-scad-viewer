/** Public polygon API. No spline representation or Manifold dependency. */
import { callGeometryRust } from './kernel'
export interface PolygonMesh {
  positions: number[]
  indices: number[]
  uv?: number[]
}
export interface PolygonReport {
  triangleCount: number
  vertexCount: number
  boundaryEdges: number
  nonManifoldEdges: number
  orientationConflicts: number
  degenerateTriangles: number
  closed: boolean
  signedVolumeMm3: number
  errorBoundCertified: false
  selfIntersectionStatus: 'not_checked' | 'checked_with_tolerance'
  boolean?: PolygonBooleanReport
  construction: 'boolean' | 'triangle_mesh' | 'sampled_surface' | 'fixed_vector_thickening'
  uvArea?: number
  sampledDeviationMm?: number
  parameterSeamsWelded?: { u: boolean; v: boolean }
  collapsedBoundaryCount?: number
}
export interface PolygonBuild extends PolygonMesh { report: PolygonReport }
export function inspectPolygonMesh(mesh: PolygonMesh): PolygonReport { return callGeometryRust('mesh_inspect', { mesh }) }
export function polygonBoundaryLoops(mesh: PolygonMesh): number[][] { return callGeometryRust('mesh_boundary_loops', { mesh }) }
export function transformPolygonMesh(mesh: PolygonMesh, matrix: number[][]): PolygonBuild { return callGeometryRust('mesh_transform', { mesh, matrix }) }
export function thickenPolygonMesh(mesh: PolygonMesh, vector: number[]): PolygonBuild { return callGeometryRust('mesh_thicken', { mesh, vector }) }
export function exportPolygonStl(mesh: PolygonMesh): string { return callGeometryRust('mesh_export_stl', { mesh }) }

export type PolygonBooleanOperation = 'union' | 'intersection' | 'difference'
export interface PolygonBooleanOptions {
  relativeTolerance?: number
  maxWork?: number
  maxFragments?: number
  maxOutputTriangles?: number
}
export interface PolygonBooleanReport {
  operation: PolygonBooleanOperation
  toleranceMm: number
  work: number
  fragments: number
  inputTriangles: [number, number]
}
export function booleanPolygonMeshes(a: PolygonMesh, b: PolygonMesh, operation: PolygonBooleanOperation, options: PolygonBooleanOptions = {}): PolygonBuild {
  return callGeometryRust('mesh_boolean', { a, b, operation, options })
}

export interface PolygonProfile {outer:number[][];holes?:number[][][]}
export const extrudePolygonProfile=(profile:PolygonProfile,vector:number[]):PolygonBuild=>callGeometryRust('polygon_extrude',{profile,vector})
export const revolvePolygonProfile=(profile:number[][],angle=360,segments=32,caps=true):PolygonBuild=>callGeometryRust('polygon_revolve',{profile,angle,segments,caps})
export const loftPolygonSections=(sections:number[][][],caps=true):PolygonBuild=>callGeometryRust('polygon_loft',{sections,caps})
export const sweepPolygonProfile=(profile:number[][],path:number[][],up=[1,0,0],caps=true):PolygonBuild=>callGeometryRust('polygon_sweep',{profile,path,up,caps})

import type {GeometryDeformation,GeometryBrush} from '../geometryEditing'
export const deformPolygonMesh=(mesh:PolygonMesh,deformation:GeometryDeformation):PolygonBuild=>callGeometryRust('polygon_deform',{mesh,deformation})
export const brushPolygonMesh=(mesh:PolygonMesh,brush:GeometryBrush):PolygonBuild=>callGeometryRust('polygon_brush',{mesh,brush})
export const extrudePolygonFaces=(mesh:PolygonMesh,triangles:number[],vector:number[]):PolygonBuild=>callGeometryRust('polygon_extrude_faces',{mesh,triangles,vector})
