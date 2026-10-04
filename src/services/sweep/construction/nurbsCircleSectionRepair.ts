import type {NurbsCurve} from '../../nurbsCurve'
import {callNurbsRust} from '../../geometry/nurbs'
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
 checkAbort()
 const report=callNurbsRust<CircleSweepSectionRepair>('sweep_repair_circle_sections',{sections,...options})
 checkAbort()
 return report
}
