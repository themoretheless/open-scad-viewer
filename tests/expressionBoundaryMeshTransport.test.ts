import {describe,it,expect} from 'vitest'
import {validBoundaryMesh,validBoundaryMeshLimits,type BoundaryMeshReport} from '../src/services/expressionBoundaryMesh'
const limits={maxDeviationMm:0.05,cells:100,spans:1000,maxDepth:12}
const fresh=():BoundaryMeshReport=>({version:1,materialQualified:false,surfaceToleranceQualified:false,closedOrientedIncidence:false,
 openEdges:3,nonmanifoldEdges:0,orientationConflicts:0,invalidVertexLinks:3,
 vertices:[[0,0,0],[1,0,0],[0,1,0]].map(point=>({point,bounds:point.map(x=>[x,x])})),
 segments:[],pending:[],candidateTriangles:[{face:0,vertices:[0,1,2],uv:[[0,0],[1,0],[0,1]]}],pendingFaces:[]})
describe('native boundary graph transport',()=>{
 it('accepts an explicitly unqualified candidate mesh',()=>expect(validBoundaryMesh(limits,1,1,fresh())).toBe(true))
 it('refuses broken vertex references, invalid bounds and inflated qualification',()=>{
  const badIndex=fresh();badIndex.candidateTriangles[0].vertices[0]=99;expect(validBoundaryMesh(limits,1,1,badIndex)).toBe(false)
  const badBounds=fresh();badBounds.vertices[0].bounds![0]=[2,1];expect(validBoundaryMesh(limits,1,1,badBounds)).toBe(false)
  const falseClosure=fresh();falseClosure.closedOrientedIncidence=true;expect(validBoundaryMesh(limits,1,1,falseClosure)).toBe(false)
  const inflated=fresh();(inflated as any).materialQualified=true;expect(validBoundaryMesh(limits,1,1,inflated)).toBe(false)
 })
 it('refuses an edge beyond the requested deviation',()=>{
  const v=fresh();v.segments=[{pair:0,vertices:[0,1],deviationUpperMm:0.1,uv:Array.from({length:2},()=>Array.from({length:2},()=>[[0,0],[0,0]]))}]
  expect(validBoundaryMesh(limits,1,1,v)).toBe(false)
 })
})

describe('refined boundary triangle transport',()=>{
 it('refuses malformed optional refinement requests before dispatch',()=>{
  for(const refinement of [null,false,0,{}, {cells:1}])expect(validBoundaryMeshLimits({...limits,refinement} as any)).toBe(false)
 })
 const refinement={maxDeviationMm:0.05,cells:8,spans:1000,spansPerQuery:64,controls:1000,domainCells:1000,domainCellsPerTriangle:64,toleranceUv:1e-8,maxDepth:4,regionalSpans:1000}
 const requested={...limits,refinement}
 const response=()=>{
  const mesh=fresh()
  mesh.refinement={version:1,limits:{...refinement},solidQualified:false,closedOrientedIncidence:false,
   initialTriangles:1,initialChecked:1,splitTriangles:0,cells:1,spans:64,domainCells:1,
   regional:{spans:0,queries:0,accepted:0},accepted:[],pending:[{...mesh.candidateTriangles[0],reason:'triangle-query-disabled'}],numericFailures:[]}
  return mesh
 }
 it('retains unproven triangles and rejects an unsolicited refinement',()=>{
  expect(validBoundaryMesh(requested,1,1,response())).toBe(true)
  expect(validBoundaryMesh(limits,1,1,response())).toBe(false)
  expect(validBoundaryMesh(requested,1,1,fresh())).toBe(false)
 })
 it('rejects lost coverage, over-budget work and mismatched tolerance',()=>{
  const lost=response();lost.refinement!.pending=[];expect(validBoundaryMesh(requested,1,1,lost)).toBe(false)
  const work=response();work.refinement!.regional.spans=1001;expect(validBoundaryMesh(requested,1,1,work)).toBe(false)
  const tolerance=response();tolerance.refinement!.limits.maxDeviationMm=0.1;expect(validBoundaryMesh(requested,1,1,tolerance)).toBe(false)
 })
 it('admits bounded native leaves without inflating solid qualification',()=>{
  const mesh=response();mesh.refinement!.accepted=[{...mesh.refinement!.pending[0],deviationUpperMm:0.04}];mesh.refinement!.pending=[]
  expect(validBoundaryMesh(requested,1,1,mesh)).toBe(true)
  mesh.refinement!.accepted[0].deviationUpperMm=0.051;expect(validBoundaryMesh(requested,1,1,mesh)).toBe(false)
  mesh.refinement!.accepted[0].deviationUpperMm=0.04;(mesh.refinement as any).solidQualified=true
  expect(validBoundaryMesh(requested,1,1,mesh)).toBe(false)
 })
})
