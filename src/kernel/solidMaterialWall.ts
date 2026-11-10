import {materialExpectation,validMaterial,type MaterialOptions,type MaterialChord,type NormalAudit} from './solidMaterialVolume'
import {callGeometryRust} from './geometry/kernel'
export interface MaterialWallOptions extends MaterialOptions {normalAudit:NormalAudit;faceGroups:[number[],number[]];toleranceMm:number;maxDistanceCells:number;maxDistanceDomainCells:number}
export interface MaterialWallResult {method:'bounded-material-wall';scope:'aligned-material-chords-between-selected-face-unions';faceGroups:[number[],number[]];toleranceMm:number;maxDistanceCells:number;maxDistanceDomainCells:number;intervalMm:[number,number]|null;converged:boolean;reason:string;candidate:MaterialChord;clearance:{lowerBoundMm:number;upperBoundMm:number|null;totalPairs:number;evaluatedPairs:number;cells:number;domainCells:number;converged:boolean;reason:string}}
const work=(x:unknown,max:number)=>typeof x==='number'&&Number.isSafeInteger(x)&&x>=0&&x<=max
export function materialWallExpectation(o:MaterialWallOptions){
 const groups=structuredClone(o.faceGroups)
 const valid=Array.isArray(groups)&&groups.length===2&&groups.every(g=>Array.isArray(g)&&g.length>0&&new Set(g).size===g.length&&g.every(i=>work(i,o.model.faces.length-1)))&&!groups[0].some(i=>groups[1].includes(i))&&groups[0].length*groups[1].length<=100000
 return {material:materialExpectation(o,'chord'),groups,valid,toleranceMm:o.toleranceMm,maxDistanceCells:o.maxDistanceCells,maxDistanceDomainCells:o.maxDistanceDomainCells}
}
export function validMaterialWall(e:ReturnType<typeof materialWallExpectation>,value:unknown):value is MaterialWallResult{
 const r=value as MaterialWallResult
 if(!e.valid||!e.material.normalAudit||!Number.isFinite(e.toleranceMm)||e.toleranceMm<=0||!work(e.maxDistanceCells,1000000)||e.maxDistanceCells<1||!work(e.maxDistanceDomainCells,8000000)||e.maxDistanceDomainCells<1)return false
 if(!r||r.method!=='bounded-material-wall'||r.scope!=='aligned-material-chords-between-selected-face-unions'||JSON.stringify(r.faceGroups)!==JSON.stringify(e.groups)||r.toleranceMm!==e.toleranceMm||r.maxDistanceCells!==e.maxDistanceCells||r.maxDistanceDomainCells!==e.maxDistanceDomainCells||typeof r.converged!=='boolean'||!validMaterial(e.material,r.candidate)||r.candidate.method!=='continuous-material-chord')return false
 const d=r.clearance,c=r.candidate
 if(!d||!Number.isFinite(d.lowerBoundMm)||d.lowerBoundMm<0||d.upperBoundMm!==null&&(!Number.isFinite(d.upperBoundMm)||d.upperBoundMm<d.lowerBoundMm)||d.totalPairs!==e.groups[0].length*e.groups[1].length||!work(d.evaluatedPairs,d.totalPairs)||!work(d.cells,e.maxDistanceCells)||!work(d.domainCells,e.maxDistanceDomainCells)||typeof d.converged!=='boolean'||typeof d.reason!=='string')return false
 if(d.converged!==(d.upperBoundMm!==null&&d.upperBoundMm-d.lowerBoundMm<=e.toleranceMm))return false
 let reason='material-chord-unproven'
 if(c.proven){
  if(c.normalAlignment!=='angular-tolerance')reason=c.normalAlignment==='oblique'?'candidate-oblique':'candidate-normal-unresolved'
  else{
   const f=c.boundary.contacts.map(x=>x.face)
   reason=e.groups[0].includes(f[0])&&e.groups[1].includes(f[1])||e.groups[0].includes(f[1])&&e.groups[1].includes(f[0])?'material-thickness-bounds':'candidate-outside-groups'
  }
 }
 if(r.reason!==reason)return false
 if(reason!=='material-thickness-bounds')return r.intervalMm===null&&!r.converged
 const interval=r.intervalMm,upper=c.lengthIntervalMm![1]
 return Array.isArray(interval)&&interval.length===2&&interval[0]===d.lowerBoundMm&&interval[1]===upper&&interval[0]<=interval[1]&&r.converged===(interval[1]-interval[0]<e.toleranceMm)
}
export const inspectMaterialWall=(options:MaterialWallOptions):MaterialWallResult=>callGeometryRust('cad_material_wall',options)
