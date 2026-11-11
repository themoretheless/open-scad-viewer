/** Historical construction evidence, never a certificate of later edited geometry. */
export interface CurveOffsetConstruction {
 version:1
 scope:'at-construction'
 sourceId:string
 distanceMm:number
 toleranceMm:number
 errorUpperMm:number
 method:'outward-rational-jets-chord-bound/1'|'outward-source-offset-bevel-wire/1'
 segmentCount:number
 crossings:number
 contacts:number
 uncertain:number
 complete:boolean
 regionTopologyCertified:false
}
export function validCurveOffsetConstruction(value:unknown):value is CurveOffsetConstruction {
 if(!value||typeof value!=='object')return false
 const v=value as CurveOffsetConstruction
 const count=(n:number)=>Number.isInteger(n)&&n>=0
 return v.version===1&&v.scope==='at-construction'&&typeof v.sourceId==='string'&&v.sourceId.length>0&&v.sourceId.length<=1024
  &&Number.isFinite(v.distanceMm)&&Number.isFinite(v.toleranceMm)&&v.toleranceMm>0
  &&Number.isFinite(v.errorUpperMm)&&v.errorUpperMm>=0&&v.errorUpperMm<=v.toleranceMm
  &&['outward-rational-jets-chord-bound/1','outward-source-offset-bevel-wire/1'].includes(v.method)
  &&count(v.segmentCount)&&v.segmentCount<=65536&&[v.crossings,v.contacts,v.uncertain].every(count)
  &&v.crossings+v.contacts+v.uncertain<=Math.min(1000000,v.segmentCount*(v.segmentCount-1)/2)
  &&typeof v.complete==='boolean'&&(!v.complete||v.uncertain===0)&&v.regionTopologyCertified===false
}
