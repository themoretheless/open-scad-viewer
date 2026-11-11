import {validTriangleRefinement,validTriangleRefinementLimits,type TriangleRefinementLimits,type TriangleRefinementReport} from './expressionTriangleRefinement'
// Transport validation only; Rust owns geometry and incidence qualification.
export interface BoundaryMeshLimits {maxDeviationMm:number;cells:number;spans:number;maxDepth:number;refinement?:TriangleRefinementLimits}
type Bounds=number[][]
export interface BoundaryMeshReport {
 refinement?:TriangleRefinementReport
 version:1;materialQualified:false;surfaceToleranceQualified:false;closedOrientedIncidence:boolean
 openEdges:number;nonmanifoldEdges:number;orientationConflicts:number;invalidVertexLinks:number
 vertices:{bounds:Bounds|null;point:number[]|null}[]
 segments:{pair:number;vertices:number[];uv:number[][][][];deviationUpperMm:number}[]
 pending:{pair:number;vertices:number[]|null;reason:string}[]
 candidateTriangles:{face:number;vertices:number[];uv:number[][]}[]
 pendingFaces:{face:number;reason:string}[]
}
const count=(v:unknown):v is number=>Number.isSafeInteger(v)&&Number(v)>=0
const finite=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v)
const vector=(v:unknown,n:number)=>Array.isArray(v)&&v.length===n&&v.every(finite)
const bounds=(v:unknown,n:number)=>Array.isArray(v)&&v.length===n&&v.every(b=>vector(b,2)&&b[0]<=b[1])
export function validBoundaryMeshLimits(v:BoundaryMeshLimits):boolean {
 if(!v||typeof v!=='object')return false
 return finite(v.maxDeviationMm)&&v.maxDeviationMm>0&&count(v.cells)&&v.cells<=100000&&count(v.spans)&&v.spans<=100000&&count(v.maxDepth)&&v.maxDepth<=32&&(v.refinement===undefined||validTriangleRefinementLimits(v.refinement))
}
export function validBoundaryMesh(limits:BoundaryMeshLimits,faces:number,pairs:number,value:unknown):value is BoundaryMeshReport {
 if(!value||typeof value!=='object')return false
 const v=value as BoundaryMeshReport
 const indices=(ids:unknown,n:number)=>Array.isArray(ids)&&ids.length===n&&ids.every(i=>count(i)&&i<v.vertices.length)
 if(v.version!==1||v.materialQualified!==false||v.surfaceToleranceQualified!==false||typeof v.closedOrientedIncidence!=='boolean')return false
 if(![v.openEdges,v.nonmanifoldEdges,v.orientationConflicts,v.invalidVertexLinks].every(count))return false
 if(!Array.isArray(v.vertices)||v.vertices.length>2*pairs+limits.cells+(limits.refinement?.cells??0)||!Array.isArray(v.segments)||v.segments.length>limits.cells||!Array.isArray(v.pending)||v.pending.length>2*pairs+2*limits.cells||!Array.isArray(v.candidateTriangles)||v.candidateTriangles.length>4*v.vertices.length||!Array.isArray(v.pendingFaces)||v.pendingFaces.length>faces)return false
 if(!v.vertices.every(p=>p&&((p.bounds===null&&p.point===null)||(bounds(p.bounds,3)&&vector(p.point,3)&&p.point!.every((x,k)=>x>=p.bounds![k][0]&&x<=p.bounds![k][1])))))return false
 if(!v.segments.every(s=>s&&count(s.pair)&&s.pair<pairs&&indices(s.vertices,2)&&finite(s.deviationUpperMm)&&s.deviationUpperMm>=0&&s.deviationUpperMm<=limits.maxDeviationMm&&Array.isArray(s.uv)&&s.uv.length===2&&s.uv.every(end=>Array.isArray(end)&&end.length===2&&end.every(chart=>bounds(chart,2)))))return false
 if(!v.pending.every(p=>p&&count(p.pair)&&p.pair<pairs&&(p.vertices===null||indices(p.vertices,2))&&typeof p.reason==='string'&&p.reason.length>0))return false
 if(!v.candidateTriangles.every(t=>t&&count(t.face)&&t.face<faces&&indices(t.vertices,3)&&new Set(t.vertices).size===3&&t.vertices.every(i=>v.vertices[i].point!==null)&&Array.isArray(t.uv)&&t.uv.length===3&&t.uv.every(p=>vector(p,2))))return false
 if(!v.pendingFaces.every(p=>p&&count(p.face)&&p.face<faces&&typeof p.reason==='string'&&p.reason.length>0)||new Set(v.pendingFaces.map(p=>p.face)).size!==v.pendingFaces.length)return false
 if(limits.refinement?!validTriangleRefinement(limits.refinement,faces,v.vertices.map(p=>p.point),v.candidateTriangles.length,v.refinement):v.refinement!==undefined)return false
 if(v.refinement?.closedOrientedIncidence&&!v.closedOrientedIncidence)return false
 return !v.closedOrientedIncidence||(v.pendingFaces.length===0&&v.pending.length===0&&v.candidateTriangles.length>0&&v.openEdges+v.nonmanifoldEdges+v.orientationConflicts+v.invalidVertexLinks===0)
}
