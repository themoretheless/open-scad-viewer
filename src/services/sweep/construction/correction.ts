import type {NurbsCurve} from '../../nurbsCurve'
import type {ProgressiveMiterOptions} from '../../nurbsConstructors'
import type {SweepSectionCorrection} from './nurbsSectionProjection'
import {callGeometryRust} from '../../geometry/kernel'

type Corrected=SweepSectionCorrection & {sections:NurbsCurve[][]}
export const correctMiterCaps=(sections:NurbsCurve[][],points:[number,number,number][],closed:boolean,capCorrection:{quantum:number;tolerance:number;maxWork:number}):Corrected=>
 callGeometryRust('brep_miter_correct_sections',{sections,points,options:{closed,capCorrection}})
export const correctMiterSections=(sections:NurbsCurve[][],points:[number,number,number][],options:ProgressiveMiterOptions,checkAbort=()=>{}):Corrected|undefined=>{
 checkAbort()
 const result=callGeometryRust<Corrected|null>('brep_miter_correct_sections',{sections,points,options})
 checkAbort()
 return result??undefined
}
