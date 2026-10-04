import type {NurbsCurve} from './nurbsCurve'
import {callNurbsRust} from './geometry/nurbs'
export interface CircleSectionRepair {curves:NurbsCurve[]|null;displacementUpper:number|null;work:number;reason:string}
/** Bounded candidate correction only: compose its displacement with the ideal
 * family bound, rebuild owned edges/caps, and revalidate geometry admission. */
export function repairNurbsCircleSection(curves:NurbsCurve[],options:{quantum:number;tolerance:number;maxWork:number}):CircleSectionRepair {
 return callNurbsRust('curve_repair_circle_section',{curves,...options})
}
export interface CircleSweepSectionRepair {sections:NurbsCurve[][]|null;wallDisplacementUpper:number|null;work:number;reason:string}
/** Shared section budget; unchanged positive rational bases extend the maximum
 * control displacement through every ruled wall interpolation. */
export function repairNurbsCircleSweepSections(sections:NurbsCurve[][],options:{quantum:number;tolerance:number;maxWork:number},checkAbort=()=>{}):CircleSweepSectionRepair {
 let work=0,upper=0
 const refuse=(reason:string):CircleSweepSectionRepair=>({sections:null,wallDisplacementUpper:null,work,reason})
 if(!Number.isSafeInteger(options.maxWork)||options.maxWork<0||options.maxWork>1000000||!sections.length||sections.length>1025||!sections[0]?.length)return refuse('invalid-budget-or-sections')
 const basis=sections[0]
 for(const station of sections){
  if(station.length!==basis.length)return refuse('incompatible-section-basis')
  for(let i=0;i<station.length;i++){
   const curve=station[i]!,first=basis[i]!
   if(curve.degree!==first.degree||curve.controlPoints.length!==first.controlPoints.length||curve.knots.length!==first.knots.length||curve.knots.some((x,k)=>x!==first.knots[k])||curve.weights.length!==first.weights.length||curve.weights.some((x,k)=>x!==first.weights[k]||!Number.isFinite(x)||x<=0))return refuse('incompatible-section-basis')
  }
 }
 const corrected:NurbsCurve[][]=[]
 for(const station of sections){
  checkAbort()
  const report=repairNurbsCircleSection(station,{...options,maxWork:options.maxWork-work})
  if(!Number.isSafeInteger(report.work)||report.work<0||report.work>options.maxWork-work)throw new Error('Invalid native circle correction work accounting')
  work+=report.work
  if(!report.curves||report.displacementUpper===null)return refuse(report.reason)
  if(!Number.isFinite(report.displacementUpper)||report.displacementUpper<0||report.displacementUpper>options.tolerance)return refuse('invalid-native-displacement')
  upper=Math.max(upper,report.displacementUpper);corrected.push(report.curves)
 }
 checkAbort()
 return {sections:corrected,wallDisplacementUpper:upper,work,reason:'bounded-circle-section-interpolation'}
}
