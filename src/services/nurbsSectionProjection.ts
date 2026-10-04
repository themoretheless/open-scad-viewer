import type {NurbsCurve} from './nurbsCurve'
import {callNurbsRust} from './geometry/nurbs'
export interface NurbsSectionProjection {
 curves:NurbsCurve[]|null
 displacementUpper:number|null
 exactPlanar:boolean
 work:number
 reason:string
}
/** Explicit correction; the caller must compose displacementUpper with its
 * authored-family error budget before admitting corrected geometry. */
export function projectNurbsSection(curves:NurbsCurve[],plane:{axis:0|1|2;coefficients:[number,number];offset:number},options:{quantum:number;tolerance:number;maxWork:number}):NurbsSectionProjection {
 return callNurbsRust('curve_project_section',{curves,...plane,...options})
}

/** Candidate cap plane follows the independently parameterized authored axis.
 * Rust verifies exact retained planarity and the full rational displacement;
 * cap regions, sweep error composition and regularity remain separate gates. */
export function projectAuthoredNurbsSection(curves:NurbsCurve[],frameAxis:NurbsCurve,traversal:number,options:{quantum:number;tolerance:number;maxWork:number}):NurbsSectionProjection {
 return callNurbsRust('curve_project_section_authored_axis',{curves,frameAxis,traversal,...options})
}

export interface SweepSectionCorrection {
 sections:NurbsCurve[][]|null
 wallDisplacementUpper:number|null
 exactPlanarSections:number[]
 work:number
 reason:'bounded-circle-section-interpolation'|'bounded-section-interpolation'|'incompatible-section-basis'|'work-limit'|'projection-unproved'
}
/** With identical positive rational bases at all stations, the whole ruled
 * interpolation displacement is a convex combination of pole displacements.
 * This bounds walls relative to the original rounded section family; authored
 * sweep interpolation and cap-region error remain separate obligations. */
export function projectSweepSections(
 sections:NurbsCurve[][],
 corrections:({section:number;quantum:number;tolerance:number}&({plane:Parameters<typeof projectNurbsSection>[1];frameAxis?:never;traversal?:never}|{frameAxis:NurbsCurve;traversal:number;plane?:never}))[],
 maxWork:number,
):SweepSectionCorrection {
 let work=0
 const refuse=(reason:SweepSectionCorrection['reason']):SweepSectionCorrection=>({sections:null,wallDisplacementUpper:null,exactPlanarSections:[],work,reason})
 if(!Number.isInteger(maxWork)||maxWork<0||maxWork>1000000||!maxWork)return refuse('work-limit')
 if(sections.length<2||sections.length>1025||!sections[0]?.length||sections[0].length>64
  ||!corrections.length||corrections.length>sections.length
  ||new Set(corrections.map(c=>c.section)).size!==corrections.length
  ||corrections.some(c=>!Number.isInteger(c.section)||c.section<0||c.section>=sections.length))return refuse('incompatible-section-basis')
 const basis=sections[0]
 for(const station of sections){
  if(station.length!==basis.length)return refuse('incompatible-section-basis')
  for(let i=0;i<station.length;i++){
   const c=station[i]!,a=basis[i]!,n=c.controlPoints.length,p=c.degree
   if(!Number.isInteger(p)||p<1||n<p+1||c.periodic!==a.periodic||c.knots.length!==n+p+1
    ||c.knots.some((v,j)=>!Number.isFinite(v)||(j>0&&v<c.knots[j-1]!))
    ||!(c.knots[p]!<c.knots[n]!)||c.weights.length!==n
    ||c.weights.some(w=>!Number.isFinite(w)||w<=0)
    ||c.controlPoints.some(v=>v.length!==3||v.some(x=>!Number.isFinite(x)))
    ||c.degree!==a.degree||c.controlPoints.length!==a.controlPoints.length
    ||c.knots.length!==a.knots.length||c.knots.some((v,j)=>v!==a.knots[j])
    ||c.weights.some((v,j)=>v!==a.weights[j]))return refuse('incompatible-section-basis')
  }
 }
 const corrected=structuredClone(sections),exactPlanarSections:number[]=[]
 let upper=0
 for(const correction of corrections){
  if(work===maxWork)return refuse('work-limit')
  const options={quantum:correction.quantum,tolerance:correction.tolerance,maxWork:maxWork-work}
  const result=correction.plane?projectNurbsSection(sections[correction.section]!,correction.plane,options):projectAuthoredNurbsSection(sections[correction.section]!,correction.frameAxis!,correction.traversal!,options)
  work+=result.work
  if(!result.exactPlanar||!result.curves||result.displacementUpper===null)return refuse(result.reason==='work-limit'?'work-limit':'projection-unproved')
  corrected[correction.section]=result.curves
  upper=Math.max(upper,result.displacementUpper)
  exactPlanarSections.push(correction.section)
 }
 return {sections:corrected,wallDisplacementUpper:upper,exactPlanarSections,work,reason:'bounded-section-interpolation'}
}
