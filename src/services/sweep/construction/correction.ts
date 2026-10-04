import {repairNurbsCircleSweepSections} from './nurbsCircleSectionRepair'
import {addCertifiedErrorUpper} from '../certificates/nurbsErrorComposition'
import {projectSweepSections,type SweepSectionCorrection} from './nurbsSectionProjection'
import type {NurbsCurve} from '../../nurbsCurve'
import {type ProgressiveMiterOptions} from '../../nurbsConstructors'

export const correctMiterCaps=(sections:NurbsCurve[][],points:[number,number,number][],closed:boolean,capCorrection:{quantum:number;tolerance:number;maxWork:number}):SweepSectionCorrection & {sections:NurbsCurve[][]}=>{
 if(closed)throw new Error('Closed miter has no caps to correct')
  const endpointPlane=(end:boolean)=>{
   const at=end?points.length-1:0,from=end?points.length-2:0,to=end?points.length-1:1
   const direction=points[to]!.map((x,k)=>x-points[from]![k])
   const axis=([0,1,2] as const).reduce((a,b)=>Math.abs(direction[a]!)>=Math.abs(direction[b]!)?a:b)
   if(!Number.isFinite(direction[axis])||direction[axis]===0)throw new Error('Cap correction needs a nonzero endpoint direction')
   const free=([0,1,2] as const).filter(k=>k!==axis)
   const coefficients=free.map(k=>-direction[k]!/direction[axis]!) as [number,number]
   const offset=points[at]![axis]-coefficients[0]*points[at]![free[0]!]-coefficients[1]*points[at]![free[1]!]
   return {axis,coefficients,offset}
  }
  const result=projectSweepSections(sections,[
   {section:0,plane:endpointPlane(false),quantum:capCorrection.quantum,tolerance:capCorrection.tolerance},
   {section:sections.length-1,plane:endpointPlane(true),quantum:capCorrection.quantum,tolerance:capCorrection.tolerance},
  ],capCorrection.maxWork)
  if(!result.sections||result.wallDisplacementUpper===null)throw new Error(`Miter cap correction unproved: ${result.reason}`)
  return {...result,sections:result.sections}
}
export const correctMiterSections=(sections:NurbsCurve[][],points:[number,number,number][],options:ProgressiveMiterOptions,checkAbort=()=>{}):(SweepSectionCorrection & {sections:NurbsCurve[][]})|undefined=>{
 if(!options.circleCorrection)return options.capCorrection?correctMiterCaps(sections,points,options.closed??false,options.capCorrection):undefined
 const circle=repairNurbsCircleSweepSections(sections,options.circleCorrection,checkAbort)
 if(!circle.sections||circle.wallDisplacementUpper===null)throw new Error(`Miter circle section correction unproved: ${circle.reason}`)
 const cap=options.capCorrection?correctMiterCaps(circle.sections,points,options.closed??false,options.capCorrection):undefined
 const upper=addCertifiedErrorUpper(cap?.wallDisplacementUpper??0,circle.wallDisplacementUpper)
 if(upper===null)throw new Error('Miter combined section correction displacement unproved')
 // Project caps last; their exact-plane evidence then belongs to the final sections.
 // Projection may disturb profile jets, which are revalidated on the rebuilt body.
 return {sections:cap?.sections??circle.sections,wallDisplacementUpper:upper,exactPlanarSections:cap?.exactPlanarSections??[],work:(cap?.work??0)+circle.work,reason:'bounded-circle-section-interpolation'}
}
