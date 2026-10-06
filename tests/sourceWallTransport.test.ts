import {expect,it} from 'vitest'
import {wallExpectation,validWall,type SourceWallOptions,type SourceWallResult} from '../src/services/sourceWallTransport'
const option=():SourceWallOptions=>({groups:[[0],[1]],minimumMm:5.9,toleranceMm:0.2,toleranceUv:1e-7,grid:3,maxAttempts:9,
 limits:{gapCells:1000,gapSpans:2000,cells:10000,domainCells:10000,normalSpans:1000,maxSineSquared:1e-6}})
const reply=(o:SourceWallOptions):SourceWallResult=>({request:structuredClone(o),qualified:true,converged:true,intervalMm:[5.99,6.01],
 reason:'source-wall-searched-thickness-bounds-qualified',clearance:{reason:'source-face-gap-qualified',cells:36,spans:72,uncertainFaces:null,uncertainUv:null},
 search:{attempts:9,refused:6,candidatesExhausted:true}})
const key=(v:unknown)=>JSON.stringify(v)
it('snapshots wall groups and limits; validates qualified scope and bounded work',()=>{
 const o=option(),e=wallExpectation(o,2),r=reply(o)
 o.groups[0][0]=1;o.limits.gapCells=1
 expect(e.groups).toEqual([[0],[1]]);expect(e.limits.gapCells).toBe(1000)
 expect(validWall(e,r,key)).toBe(true)
 for(const mutate of [(x:SourceWallResult)=>{x.request.minimumMm=1},(x:SourceWallResult)=>{x.intervalMm=null},
  (x:SourceWallResult)=>{x.clearance.cells=1001},(x:SourceWallResult)=>{x.search.refused=9},
  (x:SourceWallResult)=>{x.search.attempts=10},(x:SourceWallResult)=>{x.converged=false},
  (x:SourceWallResult)=>{x.intervalMm=[5,6]},(x:SourceWallResult)=>{x.clearance.uncertainFaces=[0,1]}]){
  const bad=structuredClone(r);mutate(bad);expect(validWall(e,bad,key)).toBe(false)
 }
 expect(validWall(undefined,r,key)).toBe(false);expect(validWall(e,undefined,key)).toBe(false)
})
it('requires explicit localized refusal and outward rounded convergence',()=>{
 const e=option(),r=reply(e)
 r.qualified=false;r.converged=false;r.intervalMm=null;r.reason='source-wall-clearance-unproven'
 r.clearance={reason:'source-face-gap-work-limit',cells:1000,spans:2000,uncertainFaces:[0,1],uncertainUv:[[[0,1],[0,1]],[[0,1],[0,1]]]}
 expect(validWall(e,r,key)).toBe(true)
 r.intervalMm=[5.99,6.01];expect(validWall(e,r,key)).toBe(false)
 const exact=reply(e);e.toleranceMm=exact.intervalMm![1]-exact.intervalMm![0];exact.request=structuredClone(e)
 expect(validWall(e,exact,key)).toBe(false);exact.converged=false;expect(validWall(e,exact,key)).toBe(true)
})
it('rejects invalid groups and search work before dispatch',()=>{
 for(const mutate of [(x:SourceWallOptions)=>{x.groups=[[0],[0]]},(x:SourceWallOptions)=>{x.groups=[[2],[1]]},
  (x:SourceWallOptions)=>{x.limits.cells=0},(x:SourceWallOptions)=>{x.maxAttempts=257},(x:SourceWallOptions)=>{x.toleranceUv=NaN}]){
  const bad=option();mutate(bad);expect(()=>wallExpectation(bad,2)).toThrow()
 }
})

it('validates original endpoint witnesses and rejects foreign faces or malformed coordinates',()=>{
 const e=option(),r=reply(e)
 r.witness={line:[[0,0,-1],[0,0,8]],lengthMm:[5.99,6.01],endpoints:[
  {face:0,parameter:[0.124,0.126],uv:[[0.4,0.6],[0.4,0.6]],worldMm:[[0,0],[0,0],[-0.01,0.01]]},
  {face:1,parameter:[0.874,0.876],uv:[[0.4,0.6],[0.4,0.6]],worldMm:[[0,0],[0,0],[5.99,6.01]]}]}
 expect(validWall(e,r,key)).toBe(true)
 for(const mutate of [(x:SourceWallResult)=>{x.witness!.endpoints[0].face=9},
  (x:SourceWallResult)=>{x.witness!.endpoints[1].parameter=[0.1,0.2]},
  (x:SourceWallResult)=>{x.witness!.endpoints[0].worldMm[0]=[NaN,1]},
  (x:SourceWallResult)=>{x.witness!.endpoints[0].uv[0]=[1,0]},
  (x:SourceWallResult)=>{x.witness!.line[1]=[0,0,0]},
  (x:SourceWallResult)=>{x.witness!.lengthMm=[5.99,7]}]){
  const bad=structuredClone(r);mutate(bad);expect(validWall(e,bad,key)).toBe(false)
 }
 r.qualified=false;expect(validWall(e,r,key)).toBe(false)
})
