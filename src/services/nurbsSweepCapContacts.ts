import type {NurbsBrep} from './geometry/brep'
import {callGeometryRust} from './geometry/kernel'
import {inspectSweepCapWalls,inspectSweepWalls,inspectSweepCapBoundary,inspectSweepCoedgeAgreement,inspectSweepCoedgeExact,inspectSweepBoundaryCoverage,type SweepCapWallAudit,type SweepCapBoundaryAudit,type SweepCoedgeAgreementAudit,type SweepCoedgeExactAudit,type SweepBoundaryCoverageAudit} from './nurbsSweepAudit'
import {reverseNurbsCurve} from './nurbsCurve'
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
 if(!capFaces.length||capFaces.length>16||model.faces.length>1024||new Set(capFaces).size!==capFaces.length||capFaces.some(id=>!Number.isInteger(id)||id<0||id>=model.faces.length))throw new Error('Invalid sweep cap face selection')
 if(!Number.isFinite(boundaryOptions.tolerance)||boundaryOptions.tolerance<0||!Number.isInteger(boundaryOptions.maxProducts)||boundaryOptions.maxProducts<0||boundaryOptions.maxProducts>100000)throw new Error('Invalid sweep cap boundary budget')
 const maxCells=boundaryOptions.maxCells??1000
 if(!Number.isInteger(maxCells)||maxCells<0||maxCells>100000||boundaryOptions.tolerance===0)throw new Error('Invalid sweep wall coedge budget or tolerance')
 const maxWork=boundaryOptions.maxWork??1000000
 if(!Number.isInteger(maxWork)||maxWork<0||maxWork>1000000)throw new Error('Invalid exact coedge work budget')
 const capSet=new Set(capFaces),wallFaces=model.faces.map((_,id)=>id).filter(id=>!capSet.has(id))
 const faceUses=model.faces.map((_,face)=>model.shells.flatMap((shell,shellId)=>shell.faces.filter(use=>use.face===face).map(use=>({shell:shellId,reversed:use.reversed}))))
 const edgeUses=new Map<number,{face:number;wire:number;coedge:number;reversed:boolean}[]>()
 for(const [faceId,face] of model.faces.entries())for(const wire of [face.outer,...face.holes])for(const [coedge,use] of model.loops[wire]!.coedges.entries()){
  const uses=edgeUses.get(use.edge)??[]
  uses.push({face:faceId,wire,coedge,reversed:use.reversed})
  edgeUses.set(use.edge,uses)
 }
 const walls=wallFaces.map(id=>model.faces[id]!.surface)
 let remainingDiagnosticWork=maxWork,remainingNativeWork=maxWork
 return capFaces.map(capFace=>{
  checkAbort()
  const cap=model.faces[capFace]!
  const edges=new Set([cap.outer,...cap.holes].flatMap(wire=>model.loops[wire]!.coedges.map(coedge=>coedge.edge)))
  const boundaries=wallFaces.map(id=>{
   const face=model.faces[id]!,labels=new Set<Boundary>()
   const bounds=[face.surface.knotsU[face.surface.degreeU]!,face.surface.knotsU[face.surface.controlPoints.length]!,face.surface.knotsV[face.surface.degreeV]!,face.surface.knotsV[face.surface.controlPoints[0]!.length]!]
   for(const wire of [face.outer,...face.holes])for(const coedge of model.loops[wire]!.coedges){
    if(!edges.has(coedge.edge))continue
    const points=coedge.pcurve.controlPoints
    for(const [axis,value,label] of [[0,bounds[0]!,'uMin'],[0,bounds[1]!,'uMax'],[1,bounds[2]!,'vMin'],[1,bounds[3]!,'vMax']] as const){
     if(points.length&&points.every(point=>point.length===2&&point[axis]===value))labels.add(label)
    }
   }
   return labels.size===1?[...labels][0]!:null
  })
  let remainingProducts=boundaryOptions.maxProducts,remainingCells=maxCells,remainingWork=remainingDiagnosticWork
  const capCoedges=[cap.outer,...cap.holes].flatMap(wire=>model.loops[wire]!.coedges.map((coedge,index)=>{
   checkAbort()
   const stored=model.edges[coedge.edge]!.curve
   const world=coedge.reversed?reverseNurbsCurve(stored):stored
   const agreement=inspectSweepCapBoundary(cap.surface,world,coedge.pcurve,{tolerance:boundaryOptions.tolerance,maxProducts:remainingProducts})
   remainingProducts-=agreement.products
   const exact=inspectSweepCoedgeExact(cap.surface,stored,coedge.pcurve,{reversed:coedge.reversed,maxWork:remainingWork})
   remainingWork-=exact.work
   const uses=edgeUses.get(coedge.edge)??[]
   const wallUses=uses.filter(use=>wallFaces.includes(use.face))
   const wallAgreements=wallUses.map(use=>{
    checkAbort()
    const agreement=inspectSweepCoedgeAgreement(model.faces[use.face]!.surface,stored,model.loops[use.wire]!.coedges[use.coedge]!.pcurve,{reversed:use.reversed,tolerance:boundaryOptions.tolerance,maxCells:remainingCells})
    remainingCells-=agreement.cells
    const exact=inspectSweepCoedgeExact(model.faces[use.face]!.surface,stored,model.loops[use.wire]!.coedges[use.coedge]!.pcurve,{reversed:use.reversed,maxWork:remainingWork})
    remainingWork-=exact.work
    const boundary=boundaries[wallFaces.indexOf(use.face)]
    const coverage=boundary?inspectSweepBoundaryCoverage(model.faces[use.face]!.surface,model.loops[use.wire]!.coedges[use.coedge]!.pcurve,boundary):null
    return {face:use.face,wire:use.wire,coedge:use.coedge,agreement,exact,coverage}
   })
   const capShells=faceUses[capFace]!,wallShells=wallUses.length===1?faceUses[wallUses[0]!.face]!:[]
   const pairedOpposite=uses.length===2&&uses.filter(use=>use.face===capFace).length===1&&wallUses.length===1&&capShells.length===1&&wallShells.length===1&&capShells[0]!.shell===wallShells[0]!.shell&&(wallUses[0]!.reversed!==wallShells[0]!.reversed)!==(coedge.reversed!==capShells[0]!.reversed)
   return {edge:coedge.edge,wire,coedge:index,wallFaces:wallUses.map(use=>use.face),pairedOpposite,agreement,exact,wallAgreements}
  }))
  remainingDiagnosticWork=remainingWork
  const audit=inspectSweepCapWalls(cap.surface,walls,boundaries,maxWalls)
  checkAbort()
  const native=inspectNativeSweepCapContacts(model,capFace,capFaces,{maxWalls,maxExactWork:remainingNativeWork,maxChartCells:1000,maxTrimPairs:100000,maxTrimCells:100000,maxTrimDomainCells:1000000})
  remainingNativeWork-=native.exactWork
  checkAbort()
  return {capFace,wallFaces,boundaries,audit,native,capCoedges,wallBoundariesCovered:capCoedges.length>0&&capCoedges.every(use=>use.wallAgreements.length===1&&use.wallAgreements[0]!.coverage?.wholeBoundaryCovered===true),capAndWallExactIdentityCertified:capCoedges.length>0&&capCoedges.every(use=>use.exact.exactIdentityCertified&&use.wallAgreements.length===1&&use.wallAgreements[0]!.exact.exactIdentityCertified),capEdgeAgreementWithinBudget:capCoedges.length>0&&capCoedges.every(use=>use.agreement.withinBudget),pairedOppositeCoedges:capCoedges.length>0&&capCoedges.every(use=>use.pairedOpposite),wallEdgeAgreementWithinTolerance:capCoedges.length>0&&capCoedges.every(use=>use.wallAgreements.length===1&&use.wallAgreements[0]!.agreement.withinTolerance)}
 })
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
 if(!capFaces.length||capFaces.length>16||new Set(capFaces).size!==capFaces.length||capFaces.some(id=>!Number.isInteger(id)||id<0||id>=model.faces.length))throw new Error('Invalid cap pair face selection')
 checkAbort()
 const audit=inspectSweepWalls(capFaces.map(id=>model.faces[id]!.surface),{...budgets,sharedBoundaries:[]})
 checkAbort()
 return {capFaces:[...capFaces],audit,unresolvedCapFaces:audit.unresolvedCharts.map(id=>capFaces[id]!),unresolvedPairs:audit.pairs.unresolved.map(pair=>({faces:[capFaces[pair.patches[0]]!,capFaces[pair.patches[1]]!],reason:pair.reason})),globalEmbeddingCertified:false}
}
