import {callGeometryRust} from './geometry/kernel'
import type {NurbsBrep} from './geometry/brep'
import type {NurbsCurve} from './nurbsCurve'
import {type NativeSweepCapContactBudgets} from './nurbsSweepCapContacts'

export interface SweepRetainedCorrespondence {
 exact:boolean
 wallErrorUpper:0|null
 inspectedFaces:number
 reason:'exact-retained-coefficients'|'unsupported-section-decomposition'|'retained-wall-mismatch'|'retained-wall-domain-unproved'|'retained-body-face-coverage-unproved'|'work-limit'
}

export interface SweepRetainedDecomposition {
 certified:boolean
 wallErrorUpper:number|null
 inspectedFaces:number
 products:number
 reason:string|null
}
/** Bind every actual ruled face to its original endpoint NURBS knot spans.
 * Equal positive weights along V make each face a linear endpoint blend.
 * The complete maximum then bounds every wall point; no cap/shell proof is
 * supplied. Numeric decomposition error is not relabelled as exact identity. */
export function inspectSweepRetainedDecomposition(model:NurbsBrep,sections:NurbsCurve[][][],closed:boolean,maxProducts=100000,maxFaces=1024,maxExactWork=1000000):SweepRetainedDecomposition {
 return callGeometryRust('brep_sweep_retained_decomposition_audit',{model,sections,closed,maxProducts,maxFaces,maxExactWork})
}
export interface SweepRetainedCaps {
 exact:boolean
 capErrorUpper:0|null
 exactWork:number
 faces:number[]
 reason:'exact-planar-regions'|'unsupported-section-decomposition'|'cap-contour-mismatch'|'cap-region-unproved'|'work-limit'
}
/** A simple planar region with the same oriented outer/hole contours is the
 * same material set. Native trim and chart certificates are recomputed here. */
export function inspectSweepRetainedCaps(model:NurbsBrep,endpoints:[NurbsCurve[][],NurbsCurve[][]],budgets:NativeSweepCapContactBudgets,maxEdges=1024):SweepRetainedCaps {
 return callGeometryRust('brep_sweep_retained_caps_audit',{model,endpoints,...budgets,maxEdges})
}
/** Bind the interpolation family to actual retained walls. This proves zero
 * extra wall error only for already segmented Bezier sections; caps and the
 * authored-family interpolation bound are separate obligations. */
export function inspectSweepRetainedCorrespondence(model:NurbsBrep,sections:NurbsCurve[][][],closed:boolean,maxFaces=1024,maxExactWork=1000000):SweepRetainedCorrespondence {
 return callGeometryRust('brep_sweep_retained_correspondence_audit',{model,sections,closed,maxFaces,maxExactWork})
}

export interface SweepRetainedCapDecomposition {
 certified:boolean
 capErrorUpper:[number,number]|null
 products:number
 regions:SweepRetainedCaps|null
 reason:string|null
}
/** Certify original endpoint spans against the actual decomposed cap contours.
 * Region identity is established only for the decomposed curves, with the
 * original outer/hole partition preserved. Both endpoints share one budget. */
export function inspectSweepRetainedCapDecomposition(model:NurbsBrep,endpoints:[NurbsCurve[][],NurbsCurve[][]],budgets:NativeSweepCapContactBudgets,maxProducts=100000,maxEdges=1024):SweepRetainedCapDecomposition {
 return callGeometryRust('brep_sweep_retained_cap_decomposition_audit',{model,endpoints,...budgets,maxProducts,maxEdges})
}
