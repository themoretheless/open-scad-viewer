/** Explicit reconstruction: source meshes remain available to callers. */
import {callGeometryRust} from './kernel'
import type {PolygonMesh,PolygonBuild} from './polygon'
import type {SubdivisionCage} from './subdivision'
import type {SdfField} from './sdf'
import type {NurbsBrep} from './brep'
import type {NurbsSurface} from '../nurbsSurface'
export interface SampledDeviation {sampledMaxMm:number;sampledRmsMm:number;sampleCount:number;errorBoundCertified:false}
export interface SubdivisionReconstruction {
 cage:SubdivisionCage;iterations:number;vertexResidualBeforeMm:number;vertexResidualAfterMm:number;
 deviation:SampledDeviation;correspondence:'source_triangle_topology_one_refinement_step'
}
export interface NurbsPatchSet {
 patches:NurbsSurface[];faceIds:number[];mode:'faceted'|'point_normal';
 sampledMaxDeviationMm:number;sampleCount:number;errorBoundCertified:false
}
export const meshToSdf=(mesh:PolygonMesh,signed=true):SdfField=>callGeometryRust('mesh_to_sdf',{mesh,signed})
export const meshToSubdivision=(mesh:PolygonMesh,iterations=8):SubdivisionReconstruction=>callGeometryRust('mesh_to_subdivision',{mesh,iterations})
export const meshToNurbs=(mesh:PolygonMesh,mode:NurbsPatchSet['mode']='faceted',maxDeviationMm=0):NurbsPatchSet=>callGeometryRust('mesh_to_nurbs',{mesh,mode,maxDeviationMm})
export const tessellateNurbsPatches=(patches:NurbsPatchSet,segments=2):PolygonBuild & {faceIds:number[]}=>callGeometryRust('nurbs_patches_tessellate',{patches,segments})

export const meshToNurbsBrep=(mesh:PolygonMesh):NurbsBrep=>callGeometryRust('mesh_to_nurbs_brep',{mesh})
