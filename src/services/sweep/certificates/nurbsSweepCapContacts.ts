import type {NurbsBrep} from '../../geometry/brep'
import {callGeometryRust} from '../../geometry/kernel'
import {type SweepCapWallAudit,type SweepCapBoundaryAudit,type SweepCoedgeAgreementAudit,type SweepCoedgeExactAudit,type SweepBoundaryCoverageAudit} from './nurbsSweepAudit'
type Boundary='uMin'|'uMax'|'vMin'|'vMax'
export interface SweepCapContactEvidence {
 capFace:number
 wallFaces:number[]
 boundaries:(Boundary|null)[]
 native:NativeSweepCapContacts
 audit:SweepCapWallAudit
 capEdgeAgreementWithinBudget:boolean
 pairedOppositeCoedges:boolean
 capAndWallExactIdentityCertified:boolean
 wallBoundariesCovered:boolean
 wallEdgeAgreementWithinTolerance:boolean
 capCoedges:{edge:number;wire:number;coedge:number;wallFaces:number[];pairedOpposite:boolean;agreement:SweepCapBoundaryAudit;exact:SweepCoedgeExactAudit;wallAgreements:{face:number;wire:number;coedge:number;agreement:SweepCoedgeAgreementAudit;exact:SweepCoedgeExactAudit;coverage:SweepBoundaryCoverageAudit|null}[]}[]
}
/** Incidence, shell-oriented pairing, and continuous cap/edge agreement are
 * separate evidence. Agreement within tolerance does not establish exact
 * contact ownership; global certification stays false.
 * Product and wall-composition cell budgets are shared per cap. Exact work
 * is shared across all caps independently for diagnostic and native audits. */
export function inspectSweepCapContacts(model:NurbsBrep,capFaces:number[],maxWalls:number,checkAbort=()=>{},boundaryOptions:{tolerance:number;maxProducts:number;maxCells?:number;maxWork?:number}={tolerance:1e-9,maxProducts:100000,maxCells:1000}):SweepCapContactEvidence[]{
 checkAbort()
 const report=callGeometryRust<SweepCapContactEvidence[]>('brep_sweep_cap_evidence_audit',{model,capFaces,maxWalls,boundaryOptions:{...boundaryOptions,maxCells:boundaryOptions.maxCells??1000,maxWork:boundaryOptions.maxWork??1000000}})
 checkAbort()
 return report
}

export interface NativeSweepCapContacts {
 capCertified:boolean
 planarControlHullCertified:boolean
 allCapWallContactsCertified:boolean
 separatedWalls:number[]
 allowedBoundaries:[number,number][]
 unresolvedWalls:number[]
 exactWork:number
 reason:string|null
 globalEmbeddingCertified:false
}
export interface NativeSweepCapContactBudgets {
 maxWalls:number
 maxExactWork:number
 maxChartCells:number
 maxTrimPairs:number
 maxTrimCells:number
 maxTrimDomainCells:number
}
/** Recomputes all contact prerequisites from the retained B-rep, not caller flags. */
export function inspectNativeSweepCapContacts(model:NurbsBrep,capFace:number,capFaces:number[],budgets:NativeSweepCapContactBudgets):NativeSweepCapContacts {
 return callGeometryRust('brep_sweep_cap_contacts_audit',{model,capFace,capFaces,...budgets})
}

export interface SweepCapPairEvidence {
 capFaces:number[]
 audit:import('./nurbsSweepAudit').SweepWallAudit
 unresolvedCapFaces:number[]
 unresolvedPairs:{faces:[number,number];reason:string}[]
 globalEmbeddingCertified:false
}
/** Full retained cap charts are conservative supersets of their trimmed faces. */
export function inspectSweepCapPairs(model:NurbsBrep,capFaces:number[],budgets:Omit<import('./nurbsSweepAudit').SweepWallAuditOptions,'sharedBoundaries'>,checkAbort=()=>{}):SweepCapPairEvidence {
 checkAbort()
 const report=callGeometryRust<SweepCapPairEvidence>('brep_sweep_cap_pairs_audit',{model,capFaces,budgets})
 checkAbort()
 return report
}
