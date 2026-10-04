import {it,expect} from 'vitest'
import {profileIntersectionDiagnostics} from '../src/services/profileIntersectionDiagnostics'
import type {ProfileIntersectionReport} from '../src/services/geometry/profileIntersections'
import type {NurbsCurve} from '../src/services/nurbsCurve'
const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1]})
function report(first:number,second:number):ProfileIntersectionReport {
 return {scope:'distinct-profile-segment-pairs',complete:false,totalPairs:1,visitedPairs:1,resolvedPairs:0,unvisitedPairs:0,boxesVisited:1,maxPairs:1,maxBoxes:1,toleranceMm:1e-7,
  pairs:[{first:{loop:0,curve:0},second:{loop:0,curve:1},report:{version:'nurbs-foundation/5',kind:'curve_curve',rounding:'binary64-nextafter-outward',
   coverage:{method:'test',complete:false,boxesVisited:1,resourceLimit:1,bernsteinExcluded:0,krawczykIsolated:0},evidence:{} as never,
   components:[{kind:'point',first,second,point:[1,0,0],firstInterval:[first,first],secondInterval:[second,second]}],unresolved:[{parameterBox:[0,1,0,1],reason:'resource_boundary'}]}}]}
}
it('omits exact adjacent endpoint joins while preserving unresolved coverage',()=>{
 const loops=[[line([0,0],[1,0]),line([1,0],[1,1])]]
 const r=profileIntersectionDiagnostics(loops,report(1,0))
 expect(r.points).toHaveLength(0);expect(r.ordinaryJoins).toBe(1)
 expect(r.complete).toBe(false);expect(r.unresolved).toHaveLength(1)
})
it('keeps interior contacts even if the segments also share an endpoint',()=>{
 const loops=[[line([0,0],[1,0]),line([1,0],[1,1])]]
 expect(profileIntersectionDiagnostics(loops,report(.5,.5)).points).toHaveLength(1)
})
it('keeps endpoint contacts between different contours',()=>{
 const r=report(1,0);r.pairs[0]!.second={loop:1,curve:0}
 const loops=[[line([0,0],[1,0])],[line([1,0],[1,1])]]
 expect(profileIntersectionDiagnostics(loops,r).points).toHaveLength(1)
})
it('does not infer endpoint ownership for an unclamped curve',()=>{
 const a=line([0,0],[1,0]);a.knots=[-1,0,1,2]
 expect(profileIntersectionDiagnostics([[a,line([1,0],[1,1])]],report(1,0)).points).toHaveLength(1)
})
it('preserves overlaps and both source references',()=>{
 const r=report(1,0);r.pairs[0]!.report.components=[{kind:'overlap'}]
 expect(profileIntersectionDiagnostics([[line([0,0],[1,0]),line([0,0],[1,0])]],r).overlaps)
  .toEqual([{first:{loop:0,curve:0},second:{loop:0,curve:1}}])
})
it('keeps coincident starts of adjacent definitions with incompatible traversal',()=>{
 expect(profileIntersectionDiagnostics([[line([0,0],[1,0]),line([0,0],[0,1])]],report(0,0)).points).toHaveLength(1)
})
it('retains a numerical contact enclosure at an exact join as uncertainty',()=>{
 const r=report(1-1e-12,1e-12)
 r.complete=true;r.resolvedPairs=1;r.pairs[0]!.report.coverage.complete=true;r.pairs[0]!.report.unresolved=[]
 r.pairs[0]!.report.components[0]!.geometryEnclosure=[[1-1e-7,1+1e-7],[-1e-7,1e-7],[-1e-7,1e-7]]
 const diagnostics=profileIntersectionDiagnostics([[line([0,0],[1,0]),line([1,0],[1,1])]],r)
 expect(diagnostics.points).toHaveLength(0)
 expect(diagnostics.endpointBands).toEqual([{first:{loop:0,curve:0},second:{loop:0,curve:1}}])
 expect(diagnostics.complete).toBe(false)
 expect(diagnostics.ordinaryJoins).toBe(0)
})
it('preserves uncertainty at both joins of a two-segment loop',()=>{
 const r=report(1e-12,1-1e-12)
 r.pairs[0]!.report.components[0]!.geometryEnclosure=[[-1e-7,1e-7],[-1e-7,1e-7],[-1e-7,1e-7]]
 const diagnostics=profileIntersectionDiagnostics([[line([0,0],[1,0]),line([1,0],[0,0])]],r)
 expect(diagnostics.points).toHaveLength(0);expect(diagnostics.endpointBands).toHaveLength(1)
})
