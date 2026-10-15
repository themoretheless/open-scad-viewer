import {callGeometryRust} from './kernel'
import type {NurbsCurve} from '../nurbsCurve'

export type BrepProfileFillRule = 'material-left' | 'even-odd'
export type BrepProfileBooleanOperation = 'union' | 'intersection' | 'difference' | 'xor'
/** Retained positive-weight rational Bézier region; empty material has loops=[] and areaMm2=0. */
export interface BrepProfile {
  kind: 'brep-profile'
  loops: NurbsCurve[][]
  areaMm2: number
  toleranceMm: number
  geometryStatus: 'numerical_uncertified'
}

/** Material-left input is checked as authored. Even-odd input reverses rings
 * according to complete analytic nesting; crossing/ambiguous rings are refused. */
export function validateBrepProfile(loops:NurbsCurve[][], fillRule:BrepProfileFillRule='material-left', toleranceMm=1e-7):BrepProfile {
  return callGeometryRust('brep_profile_validate',{loops,fillRule,toleranceMm})
}

/** Retains source NURBS trims; native geometry supplies every boundary decision. */
export function booleanBrepProfiles(a:BrepProfile,b:BrepProfile,operation:BrepProfileBooleanOperation,toleranceMm=Math.max(a.toleranceMm,b.toleranceMm)):BrepProfile {
  return callGeometryRust('brep_profile_boolean',{a:a.loops,b:b.loops,operation,toleranceMm})
}

/** Signed Green area of one connected retained curve loop, without a mesh. */
export function signedAreaBrepProfileLoop(loop:NurbsCurve[],toleranceMm=1e-7):number {
  return callGeometryRust('brep_profile_signed_area',{loop,toleranceMm})
}

export type AuthoredBrepProfile = {kind:'bezier';path:import('./path2d').BezierPathJson}| {kind:'circle';radius:number}|{kind:'rectangle';size:readonly number[];center:boolean}|{kind:'polygon';rings:readonly (readonly (readonly number[])[])[]}
export const authorBrepProfile=(definition:AuthoredBrepProfile):BrepProfile=>callGeometryRust('brep_profile_author',definition)

export const transformBrepProfile=(profile:BrepProfile,matrix:readonly number[]):BrepProfile=>callGeometryRust('brep_profile_transform',{loops:profile.loops,matrix,toleranceMm:profile.toleranceMm})

/** Euclidean parallel region with round joins, including holes and topology changes. */
export const offsetBrepProfile=(profile:BrepProfile,distance:number):BrepProfile=>callGeometryRust('brep_profile_offset',{loops:profile.loops,distance,toleranceMm:profile.toleranceMm})

/** Split resolved self-intersections with even-odd fill, retaining source trims. */
export const normalizeBrepProfile=(loops:NurbsCurve[][],toleranceMm=1e-7):BrepProfile=>callGeometryRust('brep_profile_normalize',{loops,toleranceMm})

/** Portable proof witnesses. Rust verifies topology of these exact stored bits;
 * area, construction history and solid validity are separate claims. */
export interface BrepProfileCertificate {
  version:number
  scope:'represented-profile-topology'
  source:string
  subdivisionDepth:number
  monotoneAxes:number[]
  separatingNormals:number[]
  orientation:number[]
  orientationDepth:number|null
  nestingDepth:number[]
}
export const certifyBrepProfile=(profile:BrepProfile):BrepProfileCertificate=>callGeometryRust('brep_profile_certify',{loops:profile.loops,toleranceMm:profile.toleranceMm})
export const verifyBrepProfileCertificate=(profile:BrepProfile,certificate:BrepProfileCertificate):{status:'verified';scope:'represented-profile-topology';spans:number;pieces:number;nestingDepth:number[]}=>callGeometryRust('brep_profile_verify_certificate',{loops:profile.loops,toleranceMm:profile.toleranceMm,certificate})
