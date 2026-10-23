import {callGeometryRust} from './geometry/kernel'
import type {NurbsBrep} from './geometry/brep'
import type {NurbsCurve} from './nurbsCurve'
import type {NativeSweepCapContactBudgets} from './nurbsSweepCapContacts'

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

export interface SweepRetainedCaps {
 exact:boolean
 capErrorUpper:0|null
 exactWork:number
 faces:number[]
 reason:'exact-planar-regions'|'unsupported-section-decomposition'|'cap-contour-mismatch'|'cap-region-unproved'|'work-limit'
}

export interface SweepRetainedCapDecomposition {
 certified:boolean
 capErrorUpper:[number,number]|null
 products:number
 regions:SweepRetainedCaps|null
 reason:string|null
}

/** Native ownership, coefficient identity and actual UV/world-edge domain proof. */
export function inspectSweepRetainedCorrespondence(model:NurbsBrep,sections:NurbsCurve[][][],closed:boolean,maxFaces=1024,maxExactWork=1000000):SweepRetainedCorrespondence {
 try {return callGeometryRust('brep_sweep_retained_correspondence_audit',{model,sections,closed,maxFaces,maxExactWork})}
 catch {return {exact:false,wallErrorUpper:null,inspectedFaces:0,reason:'retained-wall-mismatch'}}
}
/** Native complete original-span/retained-wall error with shared proof budgets. */
export function inspectSweepRetainedDecomposition(model:NurbsBrep,sections:NurbsCurve[][][],closed:boolean,maxProducts=100000,maxFaces=1024,maxExactWork=1000000):SweepRetainedDecomposition {
 try {return callGeometryRust('brep_sweep_retained_decomposition_audit',{model,sections,closed,maxProducts,maxFaces,maxExactWork})}
 catch {return {certified:false,wallErrorUpper:null,inspectedFaces:0,products:0,reason:'native-span-unproved'}}
}
/** Independently owned actual planar material regions. */
export function inspectSweepRetainedCaps(model:NurbsBrep,endpoints:[NurbsCurve[][],NurbsCurve[][]],budgets:NativeSweepCapContactBudgets,maxEdges=1024):SweepRetainedCaps {
 try {return callGeometryRust('brep_sweep_retained_caps_audit',{model,endpoints,...budgets,maxEdges})}
 catch {return {exact:false,capErrorUpper:null,exactWork:0,faces:[model.faces.length-2,model.faces.length-1],reason:'cap-contour-mismatch'}}
}
/** Native endpoint curve errors and independently owned retained regions.
 * Original contour regularity and full sweep error remain separate premises. */
export function inspectSweepRetainedCapDecomposition(model:NurbsBrep,endpoints:[NurbsCurve[][],NurbsCurve[][]],budgets:NativeSweepCapContactBudgets,maxProducts=100000,maxEdges=1024):SweepRetainedCapDecomposition {
 try {return callGeometryRust('brep_sweep_retained_cap_decomposition_audit',{model,endpoints,...budgets,maxProducts,maxEdges})}
 catch {return {certified:false,capErrorUpper:null,products:0,regions:null,reason:'native-span-unproved'}}
}
