/** Native source-span/resource policy and complete retained station layout. */
import {callGeometryRust} from './geometry/kernel'
import type {NurbsCurve} from './nurbsCurve'
import type {NurbsSurface} from './nurbsSurface'
export interface MiterBodyPlan {edges:number;spans:number;maxSteps:number}
export const planMiterBody=(loops:NurbsCurve[][],sites:number,closed:boolean,initialSteps:number,maxSteps:number):MiterBodyPlan=>
 callGeometryRust('brep_miter_body_plan',{loops,sites,closed,initialSteps,maxSteps})
export const partitionMiterSections=(sections:NurbsCurve[][],rings:number[]):NurbsCurve[][][]=>
 callGeometryRust('brep_miter_section_partition',{sections,rings})
export const ownedMiterSharpStations=(stations:number,edges:number,steps:number,closed:boolean):number[]=>
 callGeometryRust('brep_miter_sharp_stations',{stations,edges,steps,closed})
/** Candidate geometry only; complete-boundary and Solid certificates are separate. */
export const previewMiterWalls=(sections:NurbsCurve[][]):{patches:NurbsSurface[];profilePatchRanges:[number,number][]}=>
 callGeometryRust('brep_miter_wall_preview',{sections})
