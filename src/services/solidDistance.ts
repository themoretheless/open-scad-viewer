import type {NurbsBrep} from './geometry/brep'
import {callGeometryRust} from './geometry/kernel'
export interface VolumeValidityLimits {
 exactWork:number;trimPairs:number;trimCells:number;trimDomainCells:number;spans:number
 facePairs:number;faceCells:number;faceDomainCells:number;faceCellsPerPair:number;faceDomainCellsPerPair:number
 nestingPairs:number;nestingCells:number;nestingDomainCells:number;orientationCells:number;orientationDomainCells:number;orientationSpans:number
}
export interface SolidDistanceOptions {
 a:NurbsBrep;b:NurbsBrep;toleranceMm:number;toleranceUv:number
 maxPairs:number;maxContactPairs:number;maxCells:number;maxDomainCells:number;validityLimits:VolumeValidityLimits
}
type Interval=[number,number]
interface Validity {
 proven:boolean;boundaryProven:boolean;exactAgreement:boolean;exactJoins:boolean;trimValid:boolean;selfIntersectionAbsent:boolean
 nestingRolesConsistent:boolean|null;orientations:Array<{shell:number;expectedOutward:boolean;outward:boolean|null}>
}
export interface SolidDistanceResult {
 method:'certified-volume-distance';scope:'closed-material-sets';validity:[Validity,Validity]
 distanceIntervalMm:Interval|null;materialOverlap:boolean|null;converged:boolean
 reason:'volume-validity-unproven'|'shell-pairs-incomplete'|'containment-unproven'|'material-containment'|'separated-volumes'|'boundary-distance-unresolved'|'certified-boundary-contact'
 contact:{faces:[number,number];firstUv:[Interval,Interval];secondUv:[Interval,Interval];pointIntervalMm:[Interval,Interval,Interval];contractionUpper:number}|null
 totalShellPairs:number;visitedShellPairs:number;contactPairsVisited:number;cells:number;domainCells:number;toleranceMm:number;toleranceUv:number
 limits:{maxPairs:number;maxContactPairs:number;maxCells:number;maxDomainCells:number;validity:VolumeValidityLimits}
}
export function solidDistanceExpectation(o:SolidDistanceOptions){
 return {toleranceMm:o.toleranceMm,toleranceUv:o.toleranceUv,
  limits:{maxPairs:o.maxPairs,maxContactPairs:o.maxContactPairs,maxCells:o.maxCells,maxDomainCells:o.maxDomainCells,validity:{...o.validityLimits}},
  models:[o.a,o.b].map(m=>({shells:m.shells.map((_,i)=>!m.bodies.some(b=>b.innerShells.includes(i))),
   domains:m.faces.map(({surface:s})=>[[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length]],[s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0].length]]] as [Interval,Interval])}))}
}
const integer=(x:unknown,max:number):x is number=>typeof x==='number'&&Number.isSafeInteger(x)&&x>=0&&x<=max
const interval=(x:unknown):x is Interval=>Array.isArray(x)&&x.length===2&&x.every(Number.isFinite)&&x[0]<=x[1]
const nullableBool=(x:unknown)=>x===null||typeof x==='boolean'
export function validSolidDistance(e:ReturnType<typeof solidDistanceExpectation>,value:unknown):value is SolidDistanceResult {
 const r=value as SolidDistanceResult
 if(!r||r.method!=='certified-volume-distance'||r.scope!=='closed-material-sets'||r.toleranceMm!==e.toleranceMm||r.toleranceUv!==e.toleranceUv||!r.limits||!r.limits.validity)return false
 for(const k of ['maxPairs','maxContactPairs','maxCells','maxDomainCells'] as const)if(r.limits[k]!==e.limits[k])return false
 for(const k of Object.keys(e.limits.validity) as Array<keyof VolumeValidityLimits>)if(r.limits.validity[k]!==e.limits.validity[k])return false
 if(!Array.isArray(r.validity)||r.validity.length!==2||typeof r.converged!=='boolean'||!nullableBool(r.materialOverlap))return false
 for(let i=0;i<2;i++){
  const v=r.validity[i]
  if(!v||!['proven','boundaryProven','exactAgreement','exactJoins','trimValid','selfIntersectionAbsent'].every(k=>typeof v[k as keyof Validity]==='boolean')||!nullableBool(v.nestingRolesConsistent)||!Array.isArray(v.orientations)||v.orientations.length!==e.models[i].shells.length)return false
  if(v.boundaryProven&&!(v.exactAgreement&&v.exactJoins&&v.trimValid&&v.selfIntersectionAbsent))return false
  let oriented=true
  for(let j=0;j<v.orientations.length;j++){
   const o=v.orientations[j]
   if(!o||o.shell!==j||o.expectedOutward!==e.models[i].shells[j]||!nullableBool(o.outward))return false
   if((!v.boundaryProven||v.nestingRolesConsistent!==true)&&o.outward!==null)return false
   oriented&&=o.outward===o.expectedOutward
  }
  if(!v.boundaryProven&&v.nestingRolesConsistent!==null)return false
  if(v.proven!==(v.boundaryProven&&v.nestingRolesConsistent===true&&oriented))return false
 }
 const total=e.models[0].shells.length*e.models[1].shells.length
 if(r.totalShellPairs!==total||!integer(r.visitedShellPairs,Math.min(total,e.limits.maxPairs))||!integer(r.contactPairsVisited,Math.min(e.models[0].domains.length*e.models[1].domains.length,e.limits.maxContactPairs))||!integer(r.cells,e.limits.maxCells)||!integer(r.domainCells,e.limits.maxDomainCells))return false
 const d=r.distanceIntervalMm
 if(d!==null&&(!interval(d)||d[0]<0))return false
 if(r.contact!==null){
  const c=r.contact
  if(!c||!Array.isArray(c.faces)||c.faces.length!==2||!c.faces.every((f,i)=>integer(f,e.models[i].domains.length-1))||r.contactPairsVisited===0)return false
  for(const [i,uv] of [c.firstUv,c.secondUv].entries())if(!Array.isArray(uv)||uv.length!==2||!uv.every((x,k)=>interval(x)&&x[0]>=e.models[i].domains[c.faces[i]][k][0]&&x[1]<=e.models[i].domains[c.faces[i]][k][1]))return false
  if(!c.firstUv.some(x=>x[0]===x[1])||!Array.isArray(c.pointIntervalMm)||c.pointIntervalMm.length!==3||!c.pointIntervalMm.every(interval)||!Number.isFinite(c.contractionUpper)||c.contractionUpper<0||c.contractionUpper>=0.5)return false
 }
 if(r.validity.some(v=>!v.proven))return r.reason==='volume-validity-unproven'&&d===null&&r.materialOverlap===null&&!r.converged&&r.contact===null&&r.visitedShellPairs===0&&r.contactPairsVisited===0&&r.cells===0&&r.domainCells===0
 if(r.reason==='volume-validity-unproven')return false
 if(r.reason==='certified-boundary-contact')return r.contact!==null&&r.materialOverlap===true&&r.converged&&d?.[0]===0&&d[1]===0
 if(r.contact!==null)return false
 if(r.reason==='material-containment')return r.visitedShellPairs===total&&r.materialOverlap===true&&r.converged&&d?.[0]===0&&d[1]===0
 if(r.reason==='separated-volumes'||r.reason==='boundary-distance-unresolved')return r.visitedShellPairs===total&&r.materialOverlap===false&&(d===null||d[0]>0)&&r.converged===(d!==null&&d[1]-d[0]<=e.toleranceMm)&&(r.reason==='separated-volumes')===r.converged
 if(r.reason==='shell-pairs-incomplete'||r.reason==='containment-unproven')return r.materialOverlap===null&&!r.converged&&(d===null||d[0]===0)&&((r.visitedShellPairs<total)===(r.reason==='shell-pairs-incomplete'))
 return false
}
export function measureSolidDistance(options:SolidDistanceOptions):SolidDistanceResult {
 return callGeometryRust<SolidDistanceResult>('cad_solid_distance',options)
}
