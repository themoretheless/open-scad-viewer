import {expect,it} from 'vitest'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import {inspectAuthoredFrameRegularity} from '../src/services/nurbsAuthoredFrameRegularity'
import {progressiveSweepNurbsProfiles,previewProgressiveNurbsProfiles,streamProgressiveNurbsProfiles,type AuthoredProgressiveSweepOptions} from '../src/services/nurbsConstructors'

const normal:NurbsCurve={degree:1,knots:[31,31,41,41],controlPoints:[[0,0,1],[0,0,1]],weights:[1,1],periodic:false}
it('certifies whole authored traversal after subdivision without promoting a sweep bound',()=>{
 const axis:NurbsCurve={degree:1,knots:[2,2,5,5],controlPoints:[[1,0,0],[-1,1,0]],weights:[1,1],periodic:false}
 const before=structuredClone({axis,normal})
 const report=inspectAuthoredFrameRegularity(axis,normal,1000)
 expect(report).toMatchObject({regularityCertified:true,continuousBound:false,reason:null})
 expect(report.certifiedIntervals).toBeGreaterThan(1)
 expect(report.cells).toBeLessThanOrEqual(1000)
 expect(inspectAuthoredFrameRegularity(axis,normal,report.cells-1).regularityCertified).toBe(false)
 expect(inspectAuthoredFrameRegularity(axis,normal,0)).toMatchObject({regularityCertified:false,cells:0,continuousBound:false})
 expect({axis,normal}).toEqual(before)
})
it('refuses a hidden interior longitudinal zero despite nonzero quarter-grid stations',()=>{
 const axis:NurbsCurve={degree:2,knots:[2,2,2,10,10,10],controlPoints:[[9/64,0,0],[-15/64,0,0],[25/64,0,0]],weights:[1,1,1],periodic:false}
 const report=inspectAuthoredFrameRegularity(axis,normal,100)
 expect(report.regularityCertified).toBe(false)
 expect(report.continuousBound).toBe(false)
 expect(report.cells).toBeLessThanOrEqual(100)
})
it('rejects invalid work budgets at the Rust boundary',()=>{
 const axis:NurbsCurve={...normal,controlPoints:[[1,0,0],[1,0,0]]}
 for(const budget of [-1,1.5,100001,Number.NaN,Number.POSITIVE_INFINITY]){
  expect(()=>inspectAuthoredFrameRegularity(axis,normal,budget)).toThrow()
 }
})
it('certifies rational multispan laws with independent knot domains',()=>{
 const axis:NurbsCurve={degree:1,knots:[2,2,3,5,5],controlPoints:[[1,0,0],[1,1,0],[-1,1,0]],weights:[1,.5,2],periodic:false}
 const before=structuredClone(axis)
 expect(inspectAuthoredFrameRegularity(axis,normal,1000)).toMatchObject({regularityCertified:true,continuousBound:false})
 expect(axis).toEqual(before)
})
it('carries the separate regularity premise through construction, preview and stream reports',async()=>{
 const profile:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[1,0,0],[2,0,0]],weights:[1,1],periodic:false}
 const path:NurbsCurve={...profile,controlPoints:[[0,0,0],[0,0,10]]}
 const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const twist={...scale,values:[0,0]}
 const options:AuthoredProgressiveSweepOptions={orientation:'authored',normal:[1,0,0],maxDeviation:.01,initialSections:3,maxSections:3,
  frameAxis:{...scale,values:[[0,0,1],[0,0,1]]},frameNormal:{...scale,values:[[1,0,0],[1,0,0]]}}
 const built=progressiveSweepNurbsProfiles([profile],path,scale,twist,options)
 expect(built.report).toMatchObject({continuousBound:true,authoredFrameRegularity:{regularityCertified:true}})
 expect(built.levels.every(level=>level.authoredFrameRegularity?.regularityCertified)).toBe(true)
 const preview=previewProgressiveNurbsProfiles([profile],path,scale,twist,{...options,frameRegularityMaxCells:0},3)
 expect(preview.report).toMatchObject({continuousBound:true,authoredFrameRegularity:{regularityCertified:false,cells:0}})
 const stream=streamProgressiveNurbsProfiles([profile],path,scale,twist,options)
 const first=await stream.next()
 expect(first.done).toBe(false)
 expect(first.value.report.authoredFrameRegularity?.regularityCertified).toBe(true)
 // Public preview copies cannot rewrite the returned construction evidence.
 first.value.report.authoredFrameRegularity!.regularityCertified=false
 const final=await stream.next()
 expect(final.done).toBe(true)
 expect(final.value.report.authoredFrameRegularity?.regularityCertified).toBe(true)
})
