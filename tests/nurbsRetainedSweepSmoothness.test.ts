import {expect,it} from 'vitest'
import type {ProgressiveGuidedSurfaceSweepOptions} from '../src/services/nurbsConstructors'
import {bezierNurbsCurve,circleNurbsCurve,inspectProgressiveRetainedStationSeams} from '../src/services/nurbsConstructors'
const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
const path=bezierNurbsCurve([[0,0,0],[0,0,4]])
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
const options={normal:[1,0,0],orientation:'rmf' as const,initialSections:3,maxSections:65,maxDeviation:.01}
it('transports native exact retained G2 with whole seam and budget scope through packaged WASM',()=>{
 const proof=inspectProgressiveRetainedStationSeams([profile],path,scale,twist,options,65,2,1000000)
 expect(proof).toMatchObject({requestedOrder:2,allStationSeamsCertified:true,scope:'retained-station-seams-only',
  sourceFrameSmoothnessCertified:false,profileJoinsCertified:false,capJoinsCertified:false,solidCertified:false})
 expect(proof.seams).toHaveLength(63)
 expect(proof.seams.every(s=>s.c0Identity&&s.certified&&s.regularityCertified)).toBe(true)
 const short=inspectProgressiveRetainedStationSeams([profile],path,scale,twist,options,65,2,proof.exactWork-1)
 expect(short.allStationSeamsCertified).toBe(false)
 expect(short.exactWork).toBeLessThanOrEqual(proof.exactWork-1)
 expect(inspectProgressiveRetainedStationSeams([profile],path,scale,twist,options,65,2,0).allStationSeamsCertified).toBe(false)
})
it('does not promote moving twist interpolation to retained G1',()=>{
 const proof=inspectProgressiveRetainedStationSeams([profile],path,scale,{...twist,values:[0,30]},options,5,1,1000000)
 expect(proof.allStationSeamsCertified).toBe(false)
 expect(proof.seams.every(s=>s.c0Identity)).toBe(true)
})

it('keeps an actual closed retained RMF seam at C0 without promoting it to G1',()=>{
 const closed=circleNurbsCurve([0,0,0],[0,0,1],4)
 const proof=inspectProgressiveRetainedStationSeams([profile],closed,scale,twist,
  {...options,normal:[0,0,1],initialSections:5,maxSections:33},33,1,1000000)
 expect(proof.allStationSeamsCertified).toBe(false)
 expect(proof.seams).toHaveLength(32)
 expect(proof.seams.every(s=>s.c0Identity)).toBe(true)
 const closure=proof.seams.filter(s=>s.closure)
 expect(closure).toHaveLength(1)
 expect(closure[0]?.certified).toBe(false)
 expect(proof.sourceFrameSmoothnessCertified).toBe(false)
 expect(proof.solidCertified).toBe(false)
})


it.each([[0,1,3,7,15],[0,3,10,21,34]])('owns exact retained G2 across nonuniform station speeds %j without promoting source or Solid',(...stations)=>{
 const nonuniform={...path,degree:1,knots:[0,0,.25,.5,.75,1,1],
  controlPoints:stations.map(z=>[0,0,z]),weights:[1,1,1,1,1]}
 const opts={...options,initialSections:5,maxSections:5,maxDeviation:1}
 const before=structuredClone(nonuniform)
 const proof=inspectProgressiveRetainedStationSeams([profile],nonuniform,scale,twist,opts,5,2,1000000)
 expect(proof).toMatchObject({allStationSeamsCertified:true,requestedOrder:2,
  sourceFrameSmoothnessCertified:false,profileJoinsCertified:false,capJoinsCertified:false,solidCertified:false})
 expect(proof.seams).toHaveLength(3)
 expect(proof.seams.every(s=>s.c0Identity&&s.certified&&s.regularityCertified)).toBe(true)
 expect(proof.exactWork).toBeGreaterThan(0)
 const short=inspectProgressiveRetainedStationSeams([profile],nonuniform,scale,twist,opts,5,2,proof.exactWork-1)
 expect(short.allStationSeamsCertified).toBe(false)
 expect(short.exactWork).toBeLessThanOrEqual(proof.exactWork-1)
 const moving=inspectProgressiveRetainedStationSeams([profile],nonuniform,scale,{...twist,values:[0,30]},opts,5,1,1000000)
 expect(moving.allStationSeamsCertified).toBe(false)
 expect(moving.seams.every(s=>s.c0Identity)).toBe(true)
 expect(nonuniform).toEqual(before)
})


it.each(['rmf','fixed','fixed_normal','corrected_frenet','authored','guided'] as const)('retained nonuniform G2 remains owned in %s mode with affine laws',mode=>{
 const nonuniform={...path,degree:1,knots:[0,0,.25,.5,.75,1,1],controlPoints:[0,3,10,21,34].map(z=>[0,0,z]),weights:[1,1,1,1,1]}
 const vector=(value:[number,number,number])=>({...scale,values:[value,value]})
 const base={...options,normal:[1,0,0] as [number,number,number],initialSections:5,maxSections:5,maxDeviation:1,
  axisScale:vector([2,3,1]),centerLaw:vector([.25,0,0])}
 const opts:ProgressiveGuidedSurfaceSweepOptions=mode==='authored'
  ?{...base,orientation:'authored',frameAxis:vector([0,0,1]),frameNormal:vector([1,0,0])}
  :mode==='guided'?{...base,orientation:'rmf',orientationGuide:{...nonuniform,controlPoints:nonuniform.controlPoints.map(([x,y,z])=>[x!+1,y!,z!])}}
  :{...base,orientation:mode}
 const proof=inspectProgressiveRetainedStationSeams([profile],nonuniform,scale,twist,opts,5,2,1000000)
 expect(proof).toMatchObject({allStationSeamsCertified:true,requestedOrder:2,solidCertified:false,sourceFrameSmoothnessCertified:false})
 expect(proof.seams).toHaveLength(3)
 expect(inspectProgressiveRetainedStationSeams([profile],nonuniform,scale,twist,opts,5,2,proof.exactWork-1).allStationSeamsCertified).toBe(false)
})

it('certifies retained G2 for an applicable moving authored frame from actual surfaces',()=>{
 const nonuniform={...path,degree:1,knots:[0,0,.25,.5,.75,1,1],controlPoints:[0,1,3,7,15].map(z=>[0,0,z]),weights:[1,1,1,1,1]}
 const longitudinalProfile=bezierNurbsCurve([[0,1,0],[0,2,0]])
 const opts:ProgressiveGuidedSurfaceSweepOptions={...options,orientation:'authored',initialSections:5,maxSections:5,maxDeviation:1,
  frameAxis:{...scale,values:[[0,1,0],[0,1,0]]},frameNormal:{...scale,values:[[1,0,0],[1,0,1]]}}
 const before=structuredClone(opts)
 const proof=inspectProgressiveRetainedStationSeams([longitudinalProfile],nonuniform,scale,twist,opts,5,2,1000000)
 expect(proof).toMatchObject({allStationSeamsCertified:true,sourceFrameSmoothnessCertified:false,solidCertified:false})
 expect(proof.seams).toHaveLength(3)
 expect(proof.seams.every(s=>s.c0Identity&&s.regularityCertified&&s.certified)).toBe(true)
 expect(inspectProgressiveRetainedStationSeams([longitudinalProfile],nonuniform,scale,twist,opts,5,2,proof.exactWork-1).allStationSeamsCertified).toBe(false)
 // A transverse profile feels the moving frame and its retained strips kink.
 const sensitive=inspectProgressiveRetainedStationSeams([profile],nonuniform,scale,twist,opts,5,1,1000000)
 expect(sensitive.allStationSeamsCertified).toBe(false)
 expect(sensitive.seams.every(s=>s.c0Identity)).toBe(true)
 expect(opts).toEqual(before)
})

it('owns G2 for nonuniform Bezier profile chains and station strips without snapping',()=>{
 const chain={...profile,degree:2,knots:[0,0,0,.25,.25,.5,.5,1,1,1],
  controlPoints:[1,2.5,4,7.5,11,16.5,22].map(x=>[x,0,0]),weights:Array(7).fill(1)}
 const nonuniform={...path,degree:1,knots:[0,0,.25,.5,.75,1,1],
  controlPoints:[0,3,10,21,34].map(z=>[0,0,z]),weights:Array(5).fill(1)}
 const opts={...options,initialSections:5,maxSections:5,maxDeviation:1}
 const saved=structuredClone(chain)
 const proof=inspectProgressiveRetainedStationSeams([chain],nonuniform,scale,twist,opts,5,2,1000000)
 expect(proof).toMatchObject({allStationSeamsCertified:true,sourceFrameSmoothnessCertified:false,solidCertified:false})
 expect(proof.seams).toHaveLength(3)
 expect(proof.seams.every(s=>s.c0Identity&&s.regularityCertified)).toBe(true)
 expect(inspectProgressiveRetainedStationSeams([chain],nonuniform,scale,twist,opts,5,2,proof.exactWork-1).allStationSeamsCertified).toBe(false)
 const kink=structuredClone(chain);kink.controlPoints[3]![2]=.125
 expect(inspectProgressiveRetainedStationSeams([kink],nonuniform,scale,twist,opts,5,1,1000000).allStationSeamsCertified).toBe(false)
 expect(chain).toEqual(saved)
})
