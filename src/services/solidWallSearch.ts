import type {NurbsBrep} from './geometry/brep'
import {callGeometryRust} from './geometry/kernel'
import {evaluateNurbsSurface,type NurbsSurface,type NurbsSurfaceEvaluation} from './nurbsSurface'
type Point=[number,number,number]
export interface WallSearchCandidate {face:number;uv:[number,number];origin:Point;direction:Point}
type Evaluate=(s:NurbsSurface,u:number,v:number)=>NurbsSurfaceEvaluation
interface Query {face:number;uv:[number,number]}
/** Native proposals only; the original trimmed-volume audit still owns coverage. */
export function wallSearchCandidates(model:NurbsBrep,groups:[number[],number[]],evaluate:Evaluate=evaluateNurbsSurface,maxCandidates=64):WallSearchCandidate[]{
 return candidates(model,groups,evaluate,maxCandidates)
}
export function wholeWallSearchCandidates(model:NurbsBrep,evaluate:Evaluate=evaluateNurbsSurface,maxCandidates=64):WallSearchCandidate[]{
 return candidates(model,null,evaluate,maxCandidates)
}
function candidates(model:NurbsBrep,groups:[number[],number[]]|null,evaluate:Evaluate,maxCandidates:number):WallSearchCandidate[]{
 const request={model,groups,maxCandidates}
 if(evaluate===evaluateNurbsSurface)return callGeometryRust('cad_client_geometry',{operation:'wallCandidates',...request})
 // Keep the explicit caller-supplied evaluation hook. Native code selects the
 // bounded queries and computes proposals from the returned sample values.
 const plan=callGeometryRust<{targets:Query[];sources:Query[]}>('cad_client_geometry',{operation:'wallPlan',...request})
 const sample=({face,uv}:Query)=>{const value=evaluate(model.faces[face].surface,...uv);return {point:value.point,normal:value.normal??null}}
 return callGeometryRust('cad_client_geometry',{operation:'wallCandidates',...request,samples:{targets:plan.targets.map(sample),sources:plan.sources.map(sample)}})
}
