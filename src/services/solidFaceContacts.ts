import type {NurbsBrep} from './geometry/brep'
import type {NurbsProfileParameterization} from './sweep/certificates/nurbsSweepAudit'
import {callGeometryRust} from './geometry/kernel'
type Interval=[number,number]
type UV=[Interval,Interval]
type Cell=[UV,UV]
export interface FaceContactLimits {maxPairs:number;maxCells:number;maxDomainCells:number;cellsPerPair:number;domainCellsPerPair:number;maxBoxes:number}
export interface FaceContactWitness {firstUv:UV;secondUv:UV;pointIntervalMm:[Interval,Interval,Interval];contractionUpper:number}
export type SharedBoundaryCertificate={kind:'planar-face';edge:number;planarFace:number;sidedFace:number}|{kind:'opposite-sides';edge:number;faces:[number,number]}|{kind:'exact-hull';faces:[number,number];edges:number[];vertex:number|null;contactEnclosure:[Interval,Interval,Interval];joinedProof:JoinedContactProof|null}|JoinedSourceChartProof|SourceVolumeBoundary
export interface JoinedSourceChartProof {kind:'joined-source-charts';certified:true;faces:[number,number];edge:number;contactEnclosure:[Interval,Interval,Interval];projection:number[][];rowWeights:[number,number];profileParameterization:NurbsProfileParameterization|null;cells:number}
export interface SourceVolumeBoundary extends SourceVolumeSeparation {kind:'source-volume-boundary';edge:number}
export interface SourceVolumeSeparation {certified:true;faces:[number,number];sourceFaces:number[];projection:number[][];cells:number;principalMinorLower:[number,number,number]}
export interface JoinedContactProof {blendFace:number;wallFace:number;collapsedEnd:number;projection:number[][];proven:boolean;reason:string;cells:number;weightedBounds:number[]|null;dominanceMarginLower:number|null}
export interface FaceContactPair {faces:[number,number];status:'shared-boundary'|'pair-disjoint'|'interior-contact'|'pair-unresolved'|'periodic-trim-not-supported';sharedBoundary:SharedBoundaryCertificate|null;volumeSeparation?:SourceVolumeSeparation|null;witness:FaceContactWitness|null;cells:number;hullCells?:number;sourceChartCells?:number;domainCells:number;unresolvedBoxes:Cell[];unresolvedBoxCount:number}
export interface FaceContacts {
 method:'interval-trimmed-face-contacts';scope:'distinct-face-pairs';solidGeometryStatus:'not-certified'
 totalPairs:number;visitedPairs:number;unvisitedPairs:number;nextPair:[number,number]|null;allPairsVisited:boolean;allPairsDisjoint:boolean;allPairsClassified:boolean;sharedBoundaryPairCount:number
 contactPairCount:number;disjointPairCount:number;unresolvedPairCount:number;unresolvedBoxCount:number;exportedBoxCount:number;boxesTruncated:boolean
 cells:number;domainCells:number;toleranceUv:number;limits:FaceContactLimits;pairs:FaceContactPair[]
}
export function faceContactExpectation(model:NurbsBrep,toleranceUv:number,limits:FaceContactLimits){
 return {toleranceUv,limits:{...limits},domains:model.faces.map(f=>({uv:[[f.surface.knotsU[f.surface.degreeU],f.surface.knotsU[f.surface.controlPoints.length]],[f.surface.knotsV[f.surface.degreeV],f.surface.knotsV[f.surface.controlPoints[0].length]]] as UV,edges:[...new Set([f.outer,...f.holes].flatMap(l=>model.loops[l].coedges.map(c=>c.edge)))],vertices:[...new Set([f.outer,...f.holes].flatMap(l=>model.loops[l].coedges.flatMap(c=>model.edges[c.edge].vertices)))],periodic:!!(f.surface.periodicU||f.surface.periodicV)}))}
}
const integer=(x:unknown,max=Number.MAX_SAFE_INTEGER):x is number=>typeof x==='number'&&Number.isSafeInteger(x)&&x>=0&&x<=max
const interval=(x:unknown):x is Interval=>Array.isArray(x)&&x.length===2&&x.every(Number.isFinite)&&x[0]<=x[1]
export function validFaceContacts(e:ReturnType<typeof faceContactExpectation>,value:unknown,exactBoundaryAdmitted=false):value is FaceContacts {
 const r=value as FaceContacts
 if(!r||r.method!=='interval-trimmed-face-contacts'||r.scope!=='distinct-face-pairs'||r.solidGeometryStatus!=='not-certified'||r.toleranceUv!==e.toleranceUv||!r.limits||!Object.keys(e.limits).every(k=>r.limits[k as keyof FaceContactLimits]===e.limits[k as keyof FaceContactLimits])||!Array.isArray(r.pairs))return false
 const total=e.domains.length*(e.domains.length-1)/2
 if(r.totalPairs!==total||r.visitedPairs!==r.pairs.length||r.pairs.length>Math.min(total,e.limits.maxPairs)||r.unvisitedPairs!==total-r.pairs.length)return false
 let a=0,b=1,cells=0,domainCells=0,contacts=0,disjoint=0,shared=0,boxes=0,exported=0
 const uv=(x:unknown,face:number):x is UV=>Array.isArray(x)&&x.length===2&&x.every((d,k)=>interval(d)&&d[0]>=e.domains[face].uv[k][0]&&d[1]<=e.domains[face].uv[k][1])
 for(const p of r.pairs){
  if(!p||!Array.isArray(p.faces)||p.faces.length!==2||p.faces[0]!==a||p.faces[1]!==b||!integer(p.cells,e.limits.cellsPerPair)||!integer(p.domainCells,e.limits.domainCellsPerPair)||!integer(p.unresolvedBoxCount,p.cells+1)||!Array.isArray(p.unresolvedBoxes))return false
  const periodic=e.domains[a].periodic||e.domains[b].periodic
  const sourceCells=p.sourceChartCells??0,hullCells=p.hullCells??0
  if(!integer(sourceCells,e.limits.cellsPerPair)||!integer(hullCells,e.limits.cellsPerPair)||p.cells+sourceCells+hullCells>e.limits.cellsPerPair)return false
  const pairCells=p.cells+sourceCells+hullCells
  if(p.volumeSeparation){
   const v=p.volumeSeparation
   if(!exactBoundaryAdmitted||v.certified!==true||p.status!=='pair-disjoint'||!Array.isArray(v.faces)||v.faces.length!==2||v.faces[0]!==a||v.faces[1]!==b||!Array.isArray(v.sourceFaces)||v.sourceFaces.length!==4||new Set(v.sourceFaces).size!==4||!v.sourceFaces.includes(a)||!v.sourceFaces.includes(b)||!v.sourceFaces.every(f=>integer(f,e.domains.length-1))||!integer(v.cells,sourceCells)||v.cells===0||!Array.isArray(v.projection)||v.projection.length!==3||!v.projection.every(row=>Array.isArray(row)&&row.length===3&&row.every(Number.isFinite))||!Array.isArray(v.principalMinorLower)||v.principalMinorLower.length!==3||!v.principalMinorLower.every(x=>Number.isFinite(x)&&x>0))return false
  }
  if(p.status!=='shared-boundary'&&p.sharedBoundary!==null)return false
  if(p.status==='shared-boundary'){
   const c=p.sharedBoundary
   if(periodic||!c||p.cells!==0||p.domainCells!==0||p.unresolvedBoxCount!==0||p.witness!==null)return false
   if(c.kind==='exact-hull'){
    if(!exactBoundaryAdmitted||!Array.isArray(c.faces)||c.faces.length!==2||c.faces[0]!==a||c.faces[1]!==b||!Array.isArray(c.edges)||new Set(c.edges).size!==c.edges.length||!c.edges.every(edge=>integer(edge)&&e.domains[a].edges.includes(edge)&&e.domains[b].edges.includes(edge))||!Array.isArray(c.contactEnclosure)||c.contactEnclosure.length!==3||!c.contactEnclosure.every(interval))return false
    if(c.vertex!==null&&(!integer(c.vertex)||!e.domains[a].vertices.includes(c.vertex)||!e.domains[b].vertices.includes(c.vertex)))return false
    if(c.edges.length===0&&c.vertex===null)return false
    if(c.joinedProof!==null){
     const j=c.joinedProof
     if(!j||c.edges.length===0||!p.faces.includes(j.blendFace)||!p.faces.includes(j.wallFace)||j.blendFace===j.wallFace||![0,1].includes(j.collapsedEnd)||j.proven!==true||j.reason!=='global-joined-weighted-dominance'||j.cells!==512||!Array.isArray(j.projection)||j.projection.length!==2||!j.projection.every(row=>Array.isArray(row)&&row.length===3&&row.every(Number.isFinite))||!Array.isArray(j.weightedBounds)||j.weightedBounds.length!==4||!j.weightedBounds.every(Number.isFinite)||j.weightedBounds[0]<=0||j.weightedBounds[1]<=0||j.weightedBounds[2]<0||j.weightedBounds[3]<0||typeof j.dominanceMarginLower!=='number'||!Number.isFinite(j.dominanceMarginLower)||j.dominanceMarginLower<=0||j.dominanceMarginLower>j.weightedBounds[0]*j.weightedBounds[1]-j.weightedBounds[2]*j.weightedBounds[3]||![[[0,1,0],[1,0,-1]],[[1,0,0],[0,1,-1]]].some(basis=>basis.every((row,i)=>row.every((n,k)=>n===j.projection[i][k]))))return false
    }
   }else if(!integer(c.edge)||!e.domains[a].edges.includes(c.edge)||!e.domains[b].edges.includes(c.edge))return false
   else if(c.kind==='planar-face'){
    if(!p.faces.includes(c.planarFace)||!p.faces.includes(c.sidedFace)||c.planarFace===c.sidedFace)return false
   }else if(c.kind==='opposite-sides'){
    if(!Array.isArray(c.faces)||c.faces.length!==2||c.faces[0]!==a||c.faces[1]!==b)return false
   }else if(c.kind==='source-volume-boundary'){
    if(!exactBoundaryAdmitted||c.certified!==true||!Array.isArray(c.faces)||c.faces.length!==2||c.faces[0]!==a||c.faces[1]!==b||!Array.isArray(c.sourceFaces)||c.sourceFaces.length!==4||new Set(c.sourceFaces).size!==4||!c.sourceFaces.includes(a)||!c.sourceFaces.includes(b)||!c.sourceFaces.every(f=>integer(f,e.domains.length-1))||!integer(c.cells,sourceCells)||c.cells===0||!Array.isArray(c.projection)||c.projection.length!==3||!c.projection.every(row=>Array.isArray(row)&&row.length===3&&row.every(Number.isFinite))||!Array.isArray(c.principalMinorLower)||c.principalMinorLower.length!==3||!c.principalMinorLower.every(x=>Number.isFinite(x)&&x>0))return false
   }else if(c.kind==='joined-source-charts'){
    const parameterization=c.profileParameterization
    if(parameterization!==null&&(!parameterization||!Array.isArray(parameterization.weights)||parameterization.weights.length!==2||!parameterization.weights.every(x=>Number.isFinite(x)&&x>0)||!Number.isFinite(parameterization.derivativeLower)||parameterization.derivativeLower<=0||parameterization.derivativeLower>1))return false
    if(!exactBoundaryAdmitted||c.certified!==true||!Array.isArray(c.faces)||c.faces.length!==2||c.faces[0]!==a||c.faces[1]!==b||!integer(c.cells,sourceCells)||c.cells===0||!Array.isArray(c.projection)||c.projection.length!==2||!c.projection.every(row=>Array.isArray(row)&&row.length===3&&row.every(x=>Number.isSafeInteger(x)&&Math.abs(x)<=16))||!Array.isArray(c.rowWeights)||c.rowWeights.length!==2||!c.rowWeights.every(x=>Number.isFinite(x)&&x>0)||!Array.isArray(c.contactEnclosure)||c.contactEnclosure.length!==3||!c.contactEnclosure.every(interval))return false
   }else return false
   shared++
  }else if(p.status==='periodic-trim-not-supported'){
   if(!periodic||p.cells!==0||p.domainCells!==0||p.unresolvedBoxCount!==0||p.witness!==null)return false
  }else{
   if(periodic||pairCells===0)return false
   if(p.status==='pair-disjoint'){if(p.unresolvedBoxCount!==0||p.witness!==null)return false;disjoint++}
   else if(p.status==='interior-contact'){
    const w=p.witness
    if(!w||!Array.isArray(w.firstUv)||!w.firstUv.some(d=>interval(d)&&d[0]===d[1])||!uv(w.firstUv,a)||!uv(w.secondUv,b)||!Array.isArray(w.pointIntervalMm)||w.pointIntervalMm.length!==3||!w.pointIntervalMm.every(interval)||!Number.isFinite(w.contractionUpper)||w.contractionUpper<0||w.contractionUpper>=0.5||p.unresolvedBoxCount===0)return false
    contacts++
   }else if(p.status==='pair-unresolved'){if(p.witness!==null||(p.unresolvedBoxCount===0&&pairCells!==e.limits.cellsPerPair&&cells+pairCells!==e.limits.maxCells))return false}
   else return false
  }
  if(p.unresolvedBoxes.length!==Math.min(p.unresolvedBoxCount,e.limits.maxBoxes-exported)||!p.unresolvedBoxes.every(c=>Array.isArray(c)&&c.length===2&&uv(c[0],a)&&uv(c[1],b)))return false
  cells+=pairCells;domainCells+=p.domainCells;boxes+=p.unresolvedBoxCount;exported+=p.unresolvedBoxes.length
  b++;if(b===e.domains.length){a++;b=a+1}
 }
 const all=r.pairs.length===total
 if(!all&&r.pairs.length!==e.limits.maxPairs&&cells!==e.limits.maxCells&&domainCells!==e.limits.maxDomainCells)return false
 if(all?r.nextPair!==null:!Array.isArray(r.nextPair)||r.nextPair.length!==2||r.nextPair[0]!==a||r.nextPair[1]!==b)return false
 return cells<=e.limits.maxCells&&domainCells<=e.limits.maxDomainCells&&r.cells===cells&&r.domainCells===domainCells&&r.contactPairCount===contacts&&r.disjointPairCount===disjoint&&r.unresolvedPairCount===r.pairs.length-disjoint-shared&&r.sharedBoundaryPairCount===shared&&r.allPairsClassified===(all&&disjoint+shared===total)&&r.unresolvedBoxCount===boxes&&r.exportedBoxCount===exported&&r.boxesTruncated===(exported<boxes)&&r.allPairsVisited===all&&r.allPairsDisjoint===(all&&disjoint===total)
}
export function inspectFaceContacts(model:NurbsBrep,toleranceUv:number,limits:FaceContactLimits):FaceContacts {
 return callGeometryRust<FaceContacts>('cad_face_contacts',{model,toleranceUv,...limits})
}
