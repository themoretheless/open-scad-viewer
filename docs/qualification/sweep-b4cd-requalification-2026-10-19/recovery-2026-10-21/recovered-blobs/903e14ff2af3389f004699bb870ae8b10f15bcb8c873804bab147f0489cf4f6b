import {callGeometryRust} from './geometry/kernel'
import type {NurbsBrep} from './geometry/brep'
import type {StepIdentityReport} from './cadAnalyticStep'

export interface DirectIgesV2Certificate {
  capability:'iges-interchange/2'
  complete:true
  notes:string[]
}
export interface DirectIgesV2Report {
  identity:StepIdentityReport
  ignoredMetadata:string[]
  entityCount:number
  topologyCount:number
}
export interface DirectIgesV2Export extends DirectIgesV2Report {
  text:string
  certificate:DirectIgesV2Certificate
}
export interface DirectIgesV2Import extends DirectIgesV2Report {
  model:NurbsBrep
  certificate:DirectIgesV2Certificate
}

/** Strict 80-column direct IGES B-rep export; no constructor, AABB, or mesh route. */
export const exportDirectIgesV2=(model:NurbsBrep):DirectIgesV2Export=>
  callGeometryRust('brep_nurbs_export_iges_v2',{model})

/** Direct finite IGES topology import with explicit identity and ignored-metadata reporting. */
export const importDirectIgesV2=(text:string):DirectIgesV2Import=>
  callGeometryRust('brep_nurbs_import_iges_v2',{text})
