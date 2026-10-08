import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve,inspectProgressiveOriginalFrameSmoothness,inspectProgressiveClosedPathFrameSmoothness,inspectProgressiveClosedGuidedFrameSmoothness,previewProgressiveNurbsProfiles} from '../src/services/nurbsConstructors'
const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
const path=bezierNurbsCurve([[0,0,0],[0,0,4]])
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
const options={normal:[1,0,0],orientation:'rmf' as const,initialSections:3,maxSections:5,maxDeviation:.01}
it('transports closed original path-frame C2 and preserves independent retained/Solid scope',()=>{
 const source={degree:7,knots:[...Array(8).fill(2),...Array(8).fill(5)],
  controlPoints:[[0,0,0],[1,0,0],[2,1,0],[3,3,0],[-3,3,0],[-2,1,0],[-1,0,0],[0,0,0]],weights:Array(8).fill(1),periodic:false}
 const moving={degree:5,knots:[...Array(6).fill(7),...Array(6).fill(9)],values:[0,8,16,-16,-8,0],weights:Array(6).fill(1)}
 for(const orientation of ['rmf','fixed_normal','corrected_frenet'] as const)for(const spacing of ['parameter','arc_length'] as const){
  const opts={...options,normal:[0,0,1] as [number,number,number],orientation,spacing,lengthTolerance:.001,lengthMaxCells:100000,initialSections:5,maxSections:17,maxDeviation:1}
  const before=structuredClone(source)
  const audit=(curve=source,cells=10000,work=1000000)=>inspectProgressiveClosedPathFrameSmoothness([profile],curve,scale,moving,opts,2,cells,work)
  const proof=audit()
  expect(proof).toMatchObject({closedSourceFrameSmoothnessCertified:true,scope:'closed-original-path-frame-only',
   pathSeamCertified:false,retainedSeamsCertified:false,profileJoinsCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false})
  expect(audit(source,proof.cells-1).closedSourceFrameSmoothnessCertified).toBe(false)
  expect(audit(source,10000,proof.exactWork-1).closedSourceFrameSmoothnessCertified).toBe(false)
  expect(audit(source,0).closedSourceFrameSmoothnessCertified).toBe(false)
  expect(audit(source,10000,0).closedSourceFrameSmoothnessCertified).toBe(false)
  const damaged=structuredClone(source);damaged.controlPoints[3]![1]=3+Number.EPSILON*3
  expect(audit(damaged).closedSourceFrameSmoothnessCertified).toBe(false)
  const preview=previewProgressiveNurbsProfiles([profile],source,scale,moving,opts,5)
  expect(preview.report.closedSourceFrameSmoothness).toMatchObject({closedSourceFrameSmoothnessCertified:true,
   scope:'closed-original-path-frame-only',retainedSeamsCertified:false,continuousBound:false,solidCertified:false})
  expect(source).toEqual(before)
 }
})
it('transports closed original guided-frame C2 and preserves independent retained/Solid scope',()=>{
 const source={degree:7,knots:[...Array(8).fill(2),...Array(8).fill(5)],
  controlPoints:[[0,0,0],[1,0,0],[2,1,0],[3,3,0],[-3,3,0],[-2,1,0],[-1,0,0],[0,0,0]],weights:Array(8).fill(1),periodic:false}
 const guide={...structuredClone(source),knots:[...Array(8).fill(-3),...Array(8).fill(7)]};for(const p of guide.controlPoints)p[2]=1
 const moving={degree:5,knots:[...Array(6).fill(7),...Array(6).fill(9)],values:[0,8,16,-16,-8,0],weights:Array(6).fill(1)}
 for(const orientation of ['rmf'] as const)for(const spacing of ['parameter','arc_length'] as const){
  const opts={...options,orientationGuide:guide,normal:[0,0,1] as [number,number,number],orientation,spacing,lengthTolerance:.001,lengthMaxCells:100000,initialSections:5,maxSections:17,maxDeviation:1}
  const before=structuredClone(source)
  const audit=(curve=source,cells=10000,work=1000000)=>inspectProgressiveClosedGuidedFrameSmoothness([profile],curve,scale,moving,opts,2,cells,work)
  const proof=audit()
  expect(proof).toMatchObject({closedSourceFrameSmoothnessCertified:true,scope:'closed-original-guided-frame-only',
   pathSeamCertified:false,retainedSeamsCertified:false,profileJoinsCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false})
  expect(audit(source,proof.cells-1).closedSourceFrameSmoothnessCertified).toBe(false)
  expect(audit(source,10000,proof.exactWork-1).closedSourceFrameSmoothnessCertified).toBe(false)
  expect(audit(source,0).closedSourceFrameSmoothnessCertified).toBe(false)
  expect(audit(source,10000,0).closedSourceFrameSmoothnessCertified).toBe(false)
  const damaged=structuredClone(source);damaged.controlPoints[3]![1]=3+Number.EPSILON*3
  expect(audit(damaged).closedSourceFrameSmoothnessCertified).toBe(false)
  const damagedGuide=structuredClone(guide);damagedGuide.controlPoints[2]![2]=1+Number.EPSILON
  expect(inspectProgressiveClosedGuidedFrameSmoothness([profile],source,scale,moving,{...opts,orientationGuide:damagedGuide},2,10000,1000000).closedSourceFrameSmoothnessCertified).toBe(false)
  const preview=previewProgressiveNurbsProfiles([profile],source,scale,moving,opts,5)
  expect(preview.report.closedSourceFrameSmoothness).toMatchObject({closedSourceFrameSmoothnessCertified:true,
   scope:'closed-original-guided-frame-only',retainedSeamsCertified:false,continuousBound:false,solidCertified:false})
  expect(source).toEqual(before)
 }
})
it('qualifies forward rational and multispan line-frame C2 with separate positional and retained scope',()=>{
 const cubic={degree:3,knots:[2,2,2,2,5,5,5,5],controlPoints:[[0,0,0],[0,0,1],[0,0,7],[0,0,10]],weights:[1,2,2,1],periodic:false}
 const multispan={degree:1,knots:[2,2,3,5,5],controlPoints:[[0,0,0],[0,0,1],[0,0,10]],weights:[1,1,1],periodic:false}
 const turning={...twist,values:[0,.125]}
 for(const source of [cubic,multispan])for(const orientation of ['rmf','corrected_frenet'] as const)
  for(const spacing of ['parameter','arc_length'] as const)for(const order of [1,2]){
   const before=structuredClone(source)
   const opts={...options,normal:[1,0,1] as [number,number,number],orientation,spacing,lengthTolerance:.001,lengthMaxCells:100000}
   const audit=(cells=10000,work=1000000)=>inspectProgressiveOriginalFrameSmoothness([profile],source,scale,turning,opts,order,cells,work)
   const proof=audit()
   expect(proof).toMatchObject({sourceFrameSmoothnessCertified:true,scope:'open-original-frame-only',
    retainedSeamsCertified:false,profileJoinsCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false})
   expect(proof.cells).toBeGreaterThan(0);expect(proof.exactWork).toBeGreaterThan(0)
   expect(audit(proof.cells-1).sourceFrameSmoothnessCertified).toBe(false)
   expect(audit(10000,proof.exactWork-1).sourceFrameSmoothnessCertified).toBe(false)
   expect(audit(0).sourceFrameSmoothnessCertified).toBe(false)
   expect(audit(10000,0).sourceFrameSmoothnessCertified).toBe(false)
   if(order===2){
    const preview=previewProgressiveNurbsProfiles([profile],source,scale,turning,opts,5)
    expect(preview.report.sourceFrameSmoothness).toMatchObject({sourceFrameSmoothnessCertified:true,scope:'open-original-frame-only',
     retainedSeamsCertified:false,profileJoinsCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false})
   }
   expect(source).toEqual(before)
  }
})
it('certifies corrected planar source C1/C2 through curvature zeros with separate error and work scope',()=>{
 const ribbon=bezierNurbsCurve([[0,0,1],[0,0,2]])
 const inflection=bezierNurbsCurve([[0,0,0],[1,1,0],[2,-1,0],[3,0,0]])
 for(const rational of [false,true])for(const spacing of ['parameter','arc_length'] as const){
  const curve={...inflection,weights:rational?[1,2,2,1]:[1,1,1,1]}
  const opts={...options,normal:[0,0,1] as [number,number,number],orientation:'corrected_frenet' as const,spacing,
   lengthTolerance:.001,lengthMaxCells:100000,maxDeviation:10}
  for(const order of [1,2]){
   const audit=(source=curve,cells=10000,work=1000000)=>
    inspectProgressiveOriginalFrameSmoothness([ribbon],source,scale,twist,opts,order,cells,work)
   const proof=audit()
   expect(proof).toMatchObject({sourceFrameSmoothnessCertified:true,scope:'open-original-frame-only',
    retainedSeamsCertified:false,profileJoinsCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false})
   expect(proof.cells).toBeGreaterThan(0);expect(proof.exactWork).toBeGreaterThan(0)
   expect(audit(curve,proof.cells-1).sourceFrameSmoothnessCertified).toBe(false)
   expect(audit(curve,10000,proof.exactWork-1).sourceFrameSmoothnessCertified).toBe(false)
   expect(audit(curve,0).sourceFrameSmoothnessCertified).toBe(false)
   expect(audit(curve,10000,0).sourceFrameSmoothnessCertified).toBe(false)
   const offPlane=structuredClone(curve);offPlane.controlPoints[1]![2]=Number.MIN_VALUE
   expect(audit(offPlane).sourceFrameSmoothnessCertified).toBe(false)
  }
  const preview=previewProgressiveNurbsProfiles([ribbon],curve,scale,twist,opts,5)
  expect(preview.report).toMatchObject({continuousBound:true,
   sourceFrameSmoothness:{sourceFrameSmoothnessCertified:true,continuousBound:false,solidCertified:false}})
  expect(inspectProgressiveOriginalFrameSmoothness([ribbon],curve,scale,twist,
   {...opts,orientation:'frenet'},2,10000,1000000).sourceFrameSmoothnessCertified).toBe(false)
 }
 const stationary=bezierNurbsCurve([[0,0,0],[1,0,0],[0,0,0],[1,0,0]])
 expect(inspectProgressiveOriginalFrameSmoothness([ribbon],stationary,scale,twist,
  {...options,normal:[0,0,1],orientation:'corrected_frenet'},2,10000,1000000).sourceFrameSmoothnessCertified).toBe(false)
})
it('transports the native original frame proof without promoting surface or Solid guarantees',()=>{
 const proof=inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,options,2,10000,1000000)
 expect(proof).toMatchObject({requestedOrder:2,sourceFrameSmoothnessCertified:true,
  scope:'open-original-frame-only',retainedSeamsCertified:false,profileJoinsCertified:false,
  capJoinsCertified:false,continuousBound:false,solidCertified:false})
 expect(proof.cells).toBeGreaterThan(0)
 expect(inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,options,2,0,1000000)
  .sourceFrameSmoothnessCertified).toBe(false)
 const closed=bezierNurbsCurve([[0,0,0],[0,0,2],[0,0,0]])
 expect(inspectProgressiveOriginalFrameSmoothness([profile],closed,scale,twist,options,2,10000,1000000))
  .toMatchObject({sourceFrameSmoothnessCertified:false,reason:'original-frame-closed-seam-unproved'})
})

it('retains authored law continuity and guided correspondence limitations across WASM',()=>{
 const axis={degree:1,knots:[0,0,1,1],values:[[0,0,1],[.25,0,1]] as [number,number,number][],weights:[1,1]}
 const normal={...axis,values:[[1,0,0],[1,.25,0]] as [number,number,number][]}
 const authored={...options,orientation:'authored' as const,frameAxis:axis,frameNormal:normal}
 expect(inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,authored,2,10000,1000000)
  .sourceFrameSmoothnessCertified).toBe(true)
 const broken={degree:1,knots:[0,0,.5,1,1],values:[[1,0,0],[1,.125,0],[1,.5,0]] as [number,number,number][],weights:[1,1,1]}
 expect(inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,{...authored,frameNormal:broken},2,10000,1000000))
  .toMatchObject({sourceFrameSmoothnessCertified:false,reason:'original-frame-law-knot-continuity-unproved'})
 const guided={...options,orientationGuide:bezierNurbsCurve([[1,0,0],[1,0,4]])}
 expect(inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,guided,2,10000,1000000)
  .sourceFrameSmoothnessCertified).toBe(true)
 expect(inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,{...guided,spacing:'arc_length'},2,10000,1000000))
  .toMatchObject({sourceFrameSmoothnessCertified:true,scope:'open-original-frame-only',continuousBound:false,solidCertified:false})
})

it('checks curved FixedNormal and Frenet original frames for both station spacings',()=>{
 const curved=bezierNurbsCurve([[0,0,0],[.5,-.5,0],[1,-1,1]])
 for(const orientation of ['fixed_normal','frenet'] as const){
  for(const spacing of ['parameter','arc_length'] as const){
   const proof=inspectProgressiveOriginalFrameSmoothness([profile],curved,scale,twist,
    {...options,normal:[1,1,0],orientation,spacing,lengthTolerance:.001,lengthMaxCells:100000},2,10000,1000000)
   expect(proof.sourceFrameSmoothnessCertified,`${orientation}/${spacing}: ${proof.reason}`).toBe(true)
   expect(proof.continuousBound).toBe(false)
  }
 }
 const spatial=bezierNurbsCurve([[0,0,0],[0,0,1],[1,0,2],[1,1,3]])
 expect(inspectProgressiveOriginalFrameSmoothness([profile],spatial,scale,twist,
  {...options,orientation:'corrected_frenet'},2,10000,1000000).sourceFrameSmoothnessCertified).toBe(false)
})

it('preserves native source-frame scope in the progressive preview report',()=>{
 const preview=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,3)
 expect(preview.report.sourceFrameSmoothness).toMatchObject({sourceFrameSmoothnessCertified:true,
  scope:'open-original-frame-only',solidCertified:false,retainedSeamsCertified:false})
 const closed=circleNurbsCurve([0,0,0],[0,0,1],4)
 const refused=previewProgressiveNurbsProfiles([profile],closed,scale,twist,{...options,normal:[0,0,1],initialSections:5},5)
 expect(refused.report.sourceFrameSmoothness).toMatchObject({sourceFrameSmoothnessCertified:false,
  reason:'original-frame-closed-seam-unproved'})
})

it('transports curved independent arc-frame C2 with shared work and speed refusal',()=>{
 const curved=bezierNurbsCurve([[0,0,0],[0,0,.5],[0,1,1]])
 const rail={...bezierNurbsCurve([[1,0,0],[1,0,.5],[1,2,1]]),knots:[-3,-3,-3,7,7,7]}
 const guided={...options,orientationGuide:rail,spacing:'arc_length' as const,lengthTolerance:.001,lengthMaxCells:100000}
 const proof=inspectProgressiveOriginalFrameSmoothness([profile],curved,scale,twist,guided,2,10000,1000000)
 expect(proof).toMatchObject({sourceFrameSmoothnessCertified:true,continuousBound:false,solidCertified:false})
 expect(inspectProgressiveOriginalFrameSmoothness([profile],curved,scale,twist,guided,2,proof.cells-1,1000000)
  .sourceFrameSmoothnessCertified).toBe(false)
 const rational={...rail,weights:[1,.5,1]}
 const rationalProof=inspectProgressiveOriginalFrameSmoothness([profile],curved,scale,twist,
  {...guided,orientationGuide:rational},2,10000,1000000)
 expect(rationalProof.sourceFrameSmoothnessCertified).toBe(true)
 expect(inspectProgressiveOriginalFrameSmoothness([profile],curved,scale,twist,
  {...guided,orientationGuide:rational},2,rationalProof.cells-1,1000000).sourceFrameSmoothnessCertified).toBe(false)
 const stopped=bezierNurbsCurve([[1,0,0],[1,0,0]])
 expect(inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,{...guided,orientationGuide:stopped},2,100,1000000))
  .toMatchObject({sourceFrameSmoothnessCertified:false,reason:'guided-arc-source-speed-unproved'})
})


it('proves actual piecewise-Bezier frame jets despite a C0 basis with independent widths',()=>{
 const axis={degree:2,knots:[0,0,0,.25,.25,1,1,1],
  values:[[0,0,1],[.125,0,1],[.25,0,1],[.625,0,1],[1,0,1]] as [number,number,number][],weights:[1,1,1,1,1]}
 const normal={degree:1,knots:[-3,-3,7,7],values:[[1,0,0],[1,0,0]] as [number,number,number][],weights:[1,1]}
 const authored={...options,orientation:'authored' as const,frameAxis:axis,frameNormal:normal}
 const audit=(frameAxis=axis,order=2,work=1000000)=>inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,{...authored,frameAxis},order,10000,work)
 const proof=audit()
 expect(proof).toMatchObject({sourceFrameSmoothnessCertified:true,retainedSeamsCertified:false,continuousBound:false,solidCertified:false})
 expect(proof.exactWork).toBeGreaterThan(0)
 expect(audit(axis,2,proof.exactWork-1).sourceFrameSmoothnessCertified).toBe(false)
 const bad=structuredClone(axis);bad.values[4]![0]=1+Number.EPSILON
 expect(audit(bad).sourceFrameSmoothnessCertified).toBe(false)
 expect(audit(bad,1).sourceFrameSmoothnessCertified).toBe(true)
 const wrong={...axis,knots:[0,0,0,.5,.5,1,1,1]}
 expect(audit(wrong,1).sourceFrameSmoothnessCertified).toBe(false)
 const rational={...axis,knots:[0,0,0,.5,.5,1,1,1],values:Array.from({length:5},()=>[0,0,1] as [number,number,number]),weights:[1,.5,1,1.5,3]}
 expect(audit(rational).sourceFrameSmoothnessCertified).toBe(true)
})


it('transports exact higher path jets across mixed knots for C2 FixedNormal, Frenet, planar RMF and corrected Frenet',()=>{
 const mixed={...bezierNurbsCurve([[0,0,0],[0,0,.125],[0,.125,.375],[0,.25,.5],[0,.5,.75],[0,1,1]]),degree:2,knots:[0,0,0,.25,.5,.5,1,1,1]}
 const rational={...bezierNurbsCurve([[0,0,0],[0,0,.375],[0,.25,.625],[0,1,1]]),degree:2,knots:[0,0,0,.5,1,1,1],weights:[1,2,2,1]}
 for(const curve of [mixed,rational])for(const orientation of ['fixed_normal','frenet','rmf','corrected_frenet'] as const)for(const spacing of ['parameter','arc_length'] as const){
  const opts={...options,orientation,spacing,lengthTolerance:.001,lengthMaxCells:100000}
  const audit=(path=curve,work=1000000)=>inspectProgressiveOriginalFrameSmoothness([profile],path,scale,twist,opts,2,10000,work)
  const proof=audit()
  expect(proof,`${orientation}/${spacing}`).toMatchObject({sourceFrameSmoothnessCertified:true,continuousBound:false,retainedSeamsCertified:false,solidCertified:false})
  expect(proof.exactWork).toBeGreaterThan(0)
  expect(audit(curve,proof.exactWork-1).sourceFrameSmoothnessCertified).toBe(false)
  const bad=structuredClone(curve)
  const index=curve===mixed?3:2
  bad.controlPoints[index]![1]+=2**-54
  expect(audit(bad).sourceFrameSmoothnessCertified).toBe(false)
 }
})

it('transports closed direction-chart C1 independently of unproved C2 and retained seams',()=>{
 const source=circleNurbsCurve([0,0,0],[0,0,1],1);source.weights=source.weights.map(w=>w===1?1:.5)
 const sourceProfile=bezierNurbsCurve([[1,0,.1],[1,0,.2]])
 const before=structuredClone(source)
 for(const spacing of ['parameter','arc_length'] as const){
  const opts={...options,normal:[0,0,1],spacing,initialSections:5,maxSections:17,maxDeviation:2}
  const proof=inspectProgressiveClosedPathFrameSmoothness([sourceProfile],source,scale,twist,opts,1,10000,1000000)
  expect(proof).toMatchObject({requestedOrder:1,closedSourceFrameSmoothnessCertified:true,retainedSeamsCertified:false,continuousBound:false,solidCertified:false})
  expect(inspectProgressiveClosedPathFrameSmoothness([sourceProfile],source,scale,twist,opts,2,10000,1000000).closedSourceFrameSmoothnessCertified).toBe(false)
  expect(inspectProgressiveClosedPathFrameSmoothness([sourceProfile],source,scale,twist,opts,1,10000,proof.exactWork-1).closedSourceFrameSmoothnessCertified).toBe(false)
  const preview=previewProgressiveNurbsProfiles([sourceProfile],source,scale,twist,opts,17)
  expect(preview.report.closedSourceFrameSmoothnessC1).toMatchObject({requestedOrder:1,closedSourceFrameSmoothnessCertified:true})
 }
 expect(source).toEqual(before)
})
