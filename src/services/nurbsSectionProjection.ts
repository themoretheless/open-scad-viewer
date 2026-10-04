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
 corrections:{section:number;plane:Parameters<typeof projectNurbsSection>[1];quantum:number;tolerance:number}[],
 maxWork:number,
):SweepSectionCorrection {
 return callNurbsRust('sweep_project_sections',{sections,corrections,maxWork})
}
