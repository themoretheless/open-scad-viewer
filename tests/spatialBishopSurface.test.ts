import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs,buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {expect,it} from 'vitest'
import {bezierNurbsCurve,checkedProfileSweepNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import type {NurbsCurve} from '../src/services/nurbsCurve'
// Fixtures and assertions only; source jets, frame integration, G2 and embedding run in Rust.
const path:NurbsCurve={degree:3,knots:[0,0,0,0,.25,.25,.25,.5,.5,.5,.75,.75,.75,1,1,1,1],
 controlPoints:[[3,0,0],[3,1,.125],[1,3,.125],[0,3,0],[-1,3,-.125],[-3,1,-.125],[-3,0,0],[-3,-1,.125],[-1,-3,.125],[0,-3,0],[1,-3,-.125],[3,-1,-.125],[3,0,0]],weights:Array(13).fill(1),periodic:false}
const profile=bezierNurbsCurve([[3.125,0,0],[3.25,0,0]])
const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
it.each([6,10])('delivers a globally certified nonplanar periodic surface with exact G2 at %i stations',count=>{
 const result=checkedProfileSweepNurbsSurface(profile,path,law,[1,0,0],count,.5,65536)
 expect(result.report).toMatchObject({accepted:true,continuousBound:true,seamContinuity:'G2',seamCertificate:{exact:true,order:2},
  continuousCertificate:{method:'interval-connection-bishop-frame',withinBudget:true,globalEmbeddingCertified:false},
  geometryCertificate:{certified:true,regularity:{certified:true},embedding:{certified:true,absoluteWinding:1},solidTopologyCertified:false,pairwiseFaceContactsCertified:false}})
 const surface=result.surface!;expect(surface.periodicV).toBe(true)
 const a=evaluateNurbsSurface(surface,.37,0),b=evaluateNurbsSurface(surface,.37,1)
 for(let k=0;k<3;k++){
  expect(Math.abs(a.point[k]!-b.point[k]!)).toBeLessThan(1e-12)
  expect(Math.abs(a.dv![k]!-b.dv![k]!)).toBeLessThan(1e-10)
  expect(Math.abs(a.dvv![k]!-b.dvv![k]!)).toBeLessThan(1e-9)
 }
})
it('refuses broken authored tangents and exhausted frame work without a partial surface',()=>{
 const changed=structuredClone(path);changed.controlPoints[7]![2]!+=Number.EPSILON
 const refused=checkedProfileSweepNurbsSurface(profile,changed,law,[1,0,0],6,.5,65536)
 expect(refused.surface).toBeNull();expect(refused.report.continuousCertificate).toMatchObject({withinBudget:false,errorUpper:null,reason:'continuous-path-tangent-unproved'})
 for(const limit of [0,1,16]){
  const limited=checkedProfileSweepNurbsSurface(profile,path,law,[1,0,0],6,.5,limit)
  expect(limited.surface).toBeNull();expect(limited.report).toMatchObject({accepted:false,continuousBound:false})
 }
})

it('carries the Rust spatial-frame and surface proofs through sync and async Rush construction',async()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/spatial-bishop-g2-surface-6.r','utf8')).document
 const node=graph.nodes.find(n=>n.op==='profile_sweep')!
 const request={action:'build' as const,display:{segments:4,subdivisionLevels:0}}
 for(const built of [buildOwnNurbs(graph,request),await buildOwnNurbsAsync(graph,request)]){
  expect(built.report.construction![node.id]).toMatchObject({accepted:true,continuousBound:true,seamCertificate:{exact:true,order:2},geometryCertificate:{certified:true},continuousCertificate:{method:'interval-connection-bishop-frame'}})
 }
})
