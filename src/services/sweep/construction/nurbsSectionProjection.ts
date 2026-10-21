import type {NurbsCurve} from '../../nurbsCurve'
import {callNurbsRust} from '../../geometry/nurbs'
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

/** Project against an authored axis using the native bounded correction. */
export function projectAuthoredNurbsSection(curves:NurbsCurve[],frameAxis:NurbsCurve,traversal:number,options:{quantum:number;tolerance:number;maxWork:number}):NurbsSectionProjection {
 return callNurbsRust('curve_project_section_authored_axis',{curves,frameAxis,traversal,...options})
}

export interface SweepSectionCorrection {
 sections:NurbsCurve[][]|null
 wallDisplacementUpper:number|null
 exactPlanarSections:number[]
 work:number
 reason:'automatic-bounded-cap-planarity'|'bounded-periodic-profile-interpolation'|'bounded-rational-periodic-profile-interpolation'|'bounded-circle-section-interpolation'|'bounded-section-interpolation'|'incompatible-section-basis'|'work-limit'|'projection-unproved'
}
/** With identical positive rational bases at all stations, the whole ruled
 * interpolation displacement is a convex combination of pole displacements.
 * This bounds walls relative to the original rounded section family; authored
 * sweep interpolation and cap-region error remain separate obligations. */
export function projectSweepSections(
 sections:NurbsCurve[][],
 corrections:{section:number;plane:Parameters<typeof projectNurbsSection>[1];quantum:number;tolerance:number}[],
 maxWork:number,
):SweepSectionCorrection {
 return callNurbsRust('sweep_project_sections',{sections,corrections,maxWork})
}

export function correctMiterSweepSections(sections:NurbsCurve[][],points:[number,number,number][],options:{
 closed?:boolean;circleCorrection?:{quantum:number;tolerance:number;maxWork:number};
 capCorrection?:{quantum:number;tolerance:number;maxWork:number;authoredFrame?:boolean};
 frameAxis?:{degree:number;knots:number[];values:number[][];weights:number[]},
}):(SweepSectionCorrection & {sections:NurbsCurve[][]})|undefined {
 const budget=(value:typeof options.circleCorrection)=>value?{...value,maxWork:Number.isFinite(value.maxWork)?value.maxWork:null}:null
 const frameAxis=options.frameAxis?{degree:options.frameAxis.degree,knots:options.frameAxis.knots,controlPoints:options.frameAxis.values,weights:options.frameAxis.weights,periodic:false}:null
 return callNurbsRust<(SweepSectionCorrection & {sections:NurbsCurve[][]})|null>('sweep_correct_miter_sections',{
  sections,points,closed:options.closed??false,circleCorrection:budget(options.circleCorrection),capCorrection:budget(options.capCorrection),frameAxis,
 })??undefined
}
