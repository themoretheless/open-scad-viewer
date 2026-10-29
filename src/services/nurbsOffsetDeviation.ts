import type {NurbsSurface} from './nurbsSurface'
import {callNurbsRust} from './geometry/nurbs'

export interface OffsetDeviationOptions {
 source:NurbsSurface
 candidate:NurbsSurface
 distance:number
 tolerance:number
 maxCells:number
 maxDepth:number
}
export interface OffsetDeviationReport {
 decision:'within_tolerance'|'exceeds_tolerance'|'unresolved'
 stop:'tolerance'|'witness'|'work_limit'|'depth_limit'|'precision_limit'
 errorUpper:number|null
 errorLower:number
 witnessUv:[number,number]|null
 cells:number
 acceptedCells:number
 continuousBound:boolean
 sourceNormalsProven:boolean
 idealLocalFoldFree:boolean
 candidateLocallyRegular:boolean
 topologyAuthority:false
 globalEmbeddingCertified:false
 method:'original-span-interval-mean-value'
}
/** Checks the actual saved candidate against the original normal offset in Rust. */
export function inspectNurbsOffsetDeviation(options:OffsetDeviationOptions):OffsetDeviationReport {
 return callNurbsRust('surface_offset_continuous_deviation',options)
}
const finite=(x:unknown):x is number=>typeof x==='number'&&Number.isFinite(x)
const count=(x:unknown,max:number):x is number=>typeof x==='number'&&Number.isSafeInteger(x)&&x>=0&&x<=max
/** Wire diagnostics never carry the private native geometric owner. */
export function validOffsetDeviation(options:Pick<OffsetDeviationOptions,'maxCells'|'tolerance'>,value:unknown):value is OffsetDeviationReport {
 if(!value||typeof value!=='object'||!count(options.maxCells,100000)||!finite(options.tolerance)||options.tolerance<=0)return false
 const r=value as OffsetDeviationReport
 if(r.method!=='original-span-interval-mean-value'||r.topologyAuthority!==false||r.globalEmbeddingCertified!==false
  ||!count(r.cells,options.maxCells)||!count(r.acceptedCells,r.cells)||!finite(r.errorLower)||r.errorLower<0
  ||![r.continuousBound,r.sourceNormalsProven,r.idealLocalFoldFree,r.candidateLocallyRegular].every(x=>typeof x==='boolean')
  ||((r.idealLocalFoldFree||r.candidateLocallyRegular)&&!r.sourceNormalsProven))return false
 if(r.decision==='within_tolerance')return options.maxCells>0&&r.stop==='tolerance'&&r.continuousBound&&finite(r.errorUpper)
  &&r.errorUpper>=r.errorLower&&r.errorUpper<=options.tolerance&&r.witnessUv===null
 if(r.errorUpper!==null||r.continuousBound||r.sourceNormalsProven||r.idealLocalFoldFree||r.candidateLocallyRegular)return false
 if(r.decision==='exceeds_tolerance')return r.stop==='witness'&&r.errorLower>options.tolerance&&r.cells>0
  &&Array.isArray(r.witnessUv)&&r.witnessUv.length===2&&r.witnessUv.every(finite)
 return r.decision==='unresolved'&&['work_limit','depth_limit','precision_limit'].includes(r.stop)
  &&r.witnessUv===null&&r.errorLower<=options.tolerance
}
