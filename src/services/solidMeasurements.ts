import type {DirectBody} from './directModeling'
import {callGeometryRust} from './geometry/kernel'
export interface PointMeasurement {a:number[];b:number[];deltaMm:number[];distanceMm:number}
export interface CurveMeasurement {point:number[];curvaturePerMm:number;radiusMm:number|null;parameter:number}
export const solidVertexCount=(body:DirectBody)=>body.brep?body.brep.vertices.length:body.mesh.positions.length/3
function vertex(body:DirectBody,index:number) {
 if(!Number.isInteger(index)||index<0||index>=solidVertexCount(body))throw Error('Choose an existing vertex.')
 return body.brep?body.brep.vertices[index].point:Array.from(body.mesh.positions.slice(index*3,index*3+3))
}
export const measureSolidVertices=(a:DirectBody,ia:number,b:DirectBody,ib:number):PointMeasurement=>callGeometryRust('cad_measure_points',{a:vertex(a,ia),b:vertex(b,ib)})
export function measureSolidEdgeCurvature(body:DirectBody,edge:number,t:number):CurveMeasurement {
 if(!body.brep?.edges[edge])throw Error('Choose a B-rep edge.')
 return callGeometryRust('cad_curve_curvature',{curve:body.brep.edges[edge].curve,t})
}

export interface FaceDistanceOptions {
 a:import('./geometry/brep').NurbsBrep;b:import('./geometry/brep').NurbsBrep
 faceA:number;faceB:number;toleranceMm:number;toleranceUv:number;maxCells:number;maxDomainCells:number
}
export interface FaceDistanceResult {
 method:'interval-trimmed-surface-subdivision';scope:'trimmed-surfaces-bounded-joins'
 distanceIntervalMm:[number,number|null]
 parameters:[[number,number],[number,number]]|null
 points:[[number,number,number],[number,number,number]]|null
 pointEnclosures:[Array<[number,number]>,Array<[number,number]>]|null
 converged:boolean;reason:'tolerance'|'work-limit'|'domain-work-limit'|'precision-limit'|'empty-domain'
 cells:number;maxCells:number;domainCells:number;maxDomainCells:number;toleranceMm:number;toleranceUv:number
}
export const measureFaceDistance=(options:FaceDistanceOptions):FaceDistanceResult=>callGeometryRust('cad_face_distance',options)

export type ShellDistanceOptions=Omit<FaceDistanceOptions,'faceA'|'faceB'>
export interface ShellDistanceResult extends Omit<FaceDistanceResult,'method'|'scope'|'reason'> {
 method:'interval-trimmed-face-pairs';scope:'boundary-shells-bounded-joins';containment:'not-classified'
 faces:[number,number]|null;pairs:number;evaluatedPairs:number
 reason:'tolerance'|'work-limit'|'domain-work-limit'|'pair-resolution-limit'|'empty-domain'
}
export const measureShellDistance=(options:ShellDistanceOptions):ShellDistanceResult=>callGeometryRust('cad_shell_distance',options)
