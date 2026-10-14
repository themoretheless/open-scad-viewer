import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve,inspectProgressiveClosedAuthoredFrameSmoothness,previewProgressiveNurbsProfiles} from '../src/services/nurbsConstructors'
const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
const path=circleNurbsCurve([0,0,0],[0,0,1],4)
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
const axis={degree:5,knots:[0,0,0,0,0,0,1,1,1,1,1,1],
 values:[[0,0,1],[.125,0,1],[.25,.125,1],[-.25,.125,1],[-.125,0,1],[0,0,1]] as [number,number,number][],weights:[1,1,1,1,1,1]}
const normal={degree:1,knots:[0,0,1,1],values:[[1,0,0],[1,0,0]] as [number,number,number][],weights:[1,1]}
const options={normal:[1,0,0],orientation:'authored' as const,frameAxis:axis,frameNormal:normal,
 initialSections:5,maxSections:17,maxDeviation:2}
it('transports exact closed authored C2 separately from retained and path seams',()=>{
 const proof=inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,options,2,10000,1000000)
 expect(proof).toMatchObject({closedSourceFrameSmoothnessCertified:true,requestedOrder:2,
  scope:'closed-original-authored-frame-only',pathSeamCertified:false,retainedSeamsCertified:false,
  profileJoinsCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false})
 expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,options,2,10000,proof.exactWork-1)
  .closedSourceFrameSmoothnessCertified).toBe(false)
 expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,options,2,proof.cells-1,1000000)
  .closedSourceFrameSmoothnessCertified).toBe(false)
 // One dyadic change affects only the second endpoint jet, preserving C1.
 const wrong={...axis,values:axis.values.map(v=>[...v] as [number,number,number])}
 wrong.values[2]![0]=.25+2**-54
 const bad={...options,frameAxis:wrong}
 expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,bad,2,10000,1000000)
  .closedSourceFrameSmoothnessCertified).toBe(false)
 expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,bad,1,10000,1000000)
  .closedSourceFrameSmoothnessCertified).toBe(true)
 const projectiveBezier={...axis,weights:[1,1,1,2,2,2]}
 expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,
  {...options,frameAxis:projectiveBezier},2,10000,1000000).closedSourceFrameSmoothnessCertified).toBe(true)
 const rational={...axis,knots:[-3,-3,-3,-3,-3,-3,7,7,7,7,7,7],weights:[1,1.5,2.5,.5,.5,1],
  values:[[0,0,1],[.0625,0,1],[.125,.0625,1],[-.125,.3125,1],[-.1875,0,1],[0,0,1]] as [number,number,number][]}
 const arc={...options,frameAxis:rational,spacing:'arc_length' as const,lengthTolerance:.001,lengthMaxCells:100000}
 expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,arc,2,10000,1000000)
  .closedSourceFrameSmoothnessCertified).toBe(true)
 const open=bezierNurbsCurve([[0,0,0],[0,0,4]])
 expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],open,scale,twist,options,2,10000,1000000)
  .closedSourceFrameSmoothnessCertified).toBe(false)
})

it('attaches closed frame C2 to preview while retaining independent open-frame refusal',()=>{
 const preview=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,9)
 expect(preview.report.closedPath).toBe(true)
 expect(preview.report.closedSourceFrameSmoothness).toMatchObject({closedSourceFrameSmoothnessCertified:true,
  scope:'closed-original-authored-frame-only',pathSeamCertified:false,retainedSeamsCertified:false,solidCertified:false})
 expect(preview.report.sourceFrameSmoothness?.sourceFrameSmoothnessCertified).toBe(false)
})

it('qualifies original multispan C2 jets with knot widths and shared exact budget',()=>{
 const multi={degree:3,knots:[0,0,0,0,.25,.5,.75,1,1,1,1],
  values:[[0,0,1],[.125,0,1],[.375,.125,1],[0,.25,1],[-.375,.125,1],[-.125,0,1],[0,0,1]] as [number,number,number][],weights:[1,1,1,1,1,1,1]}
 const opts={...options,frameAxis:multi}
 const audit=(frameAxis=multi,order=2,work=1000000)=>inspectProgressiveClosedAuthoredFrameSmoothness(
  [profile],path,scale,twist,{...opts,frameAxis},order,10000,work)
 const proof=audit()
 expect(proof.closedSourceFrameSmoothnessCertified).toBe(true)
 expect(audit(multi,2,proof.exactWork-1).closedSourceFrameSmoothnessCertified).toBe(false)
 const wrong={...multi,values:multi.values.map(v=>[...v] as [number,number,number])}
 wrong.values[2]![0]=.375+2**-54
 expect(audit(wrong).closedSourceFrameSmoothnessCertified).toBe(false)
 expect(audit(wrong,1).closedSourceFrameSmoothnessCertified).toBe(true)
 const rationalMulti={...multi,weights:[1,1,1,2,1,1,1],
  knots:multi.knots.map(u=>-3+8*u)}
 expect(audit(rationalMulti).closedSourceFrameSmoothnessCertified).toBe(true)
 const projective={...multi,weights:[1,1,1,1.5,2,2,2]}
 expect(audit(projective).closedSourceFrameSmoothnessCertified).toBe(true)
 const projectiveWrong={...projective,values:projective.values.map(v=>[...v] as [number,number,number])}
 projectiveWrong.values[2]![0]=.375+2**-54
 expect(audit(projectiveWrong).closedSourceFrameSmoothnessCertified).toBe(false)
 expect(audit(projectiveWrong,1).closedSourceFrameSmoothnessCertified).toBe(true)
 const wrongWeights={...projective,weights:[1,1,1,1.5,2+2**-51,2,2]}
 expect(audit(wrongWeights).closedSourceFrameSmoothnessCertified).toBe(false)
 expect(audit(wrongWeights,1).closedSourceFrameSmoothnessCertified).toBe(true)
 const c0Only={...multi,knots:[0,0,0,0,.25,.5,.5,.5,.75,1,1,1,1],
  values:[...multi.values.slice(0,3),[0,.25,1],[0,.25,1],...multi.values.slice(3)] as [number,number,number][],weights:Array(9).fill(1)}
 expect(audit(c0Only)).toMatchObject({closedSourceFrameSmoothnessCertified:false,
  reason:'closed-authored-law-knot-continuity-unproved'})
 const altered={...multi,knots:[0,0,0,0,.125,.5,.75,1,1,1,1]}
 expect(audit(altered,1).closedSourceFrameSmoothnessCertified).toBe(false)
 expect(previewProgressiveNurbsProfiles([profile],path,scale,twist,opts,9)
  .report.closedSourceFrameSmoothness?.closedSourceFrameSmoothnessCertified).toBe(true)
})


it('audits internal C2 and closed endpoint jets independently for original piecewise laws',()=>{
 const multi={degree:5,knots:[...Array(6).fill(0),...Array(5).fill(.5),...Array(6).fill(1)],
  values:[[0,0,1],[.0625,0,1],[.125,.03125,1],[.109375,.0625,1],[.0546875,.078125,1],[0,.078125,1],[-.0546875,.078125,1],[-.109375,.0625,1],[-.125,.03125,1],[-.0625,0,1],[0,0,1]] as [number,number,number][],weights:Array(11).fill(1)}
 for(const spacing of ['parameter','arc_length'] as const){
  const audit=(frameAxis=multi,order=2,work=1000000)=>inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,{...options,spacing,frameAxis},order,10000,work)
  const proof=audit()
  expect(proof).toMatchObject({closedSourceFrameSmoothnessCertified:true,pathSeamCertified:false,retainedSeamsCertified:false,continuousBound:false,solidCertified:false})
  expect(audit(multi,2,proof.exactWork-1).closedSourceFrameSmoothnessCertified).toBe(false)
  const bad=structuredClone(multi);bad.values[7]![0]+=(2**-56)
  expect(audit(bad).closedSourceFrameSmoothnessCertified).toBe(false)
  expect(audit(bad,1).closedSourceFrameSmoothnessCertified).toBe(true)
  const preview=previewProgressiveNurbsProfiles([profile],path,scale,twist,{...options,spacing,frameAxis:multi},9)
  expect(preview.report.closedSourceFrameSmoothness?.closedSourceFrameSmoothnessCertified).toBe(true)
 }
})


it('proves Cartesian closed jets for varying homogeneous endpoint scales and nonclamped domains',()=>{
 const moving={...axis,values:[[0,0,1],[.125,0,1],[.046875,.125,1],[-.28125,.125,1],[-.03125,0,1],[0,0,1]] as [number,number,number][],weights:[1,.5,1,1,2,1]}
 const nonclamped={degree:2,knots:[-2,-1,0,.5,1,2,3],values:Array.from({length:4},()=>[0,0,1] as [number,number,number]),weights:[1,2,3,4]}
 for(const frameAxis of [moving,nonclamped])for(const spacing of ['parameter','arc_length'] as const){
  const opts={...options,frameAxis,spacing}
  const proof=inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,opts,2,10000,1000000)
  expect(proof).toMatchObject({closedSourceFrameSmoothnessCertified:true,pathSeamCertified:false,retainedSeamsCertified:false,continuousBound:false,solidCertified:false})
  expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,opts,2,10000,proof.exactWork-1).closedSourceFrameSmoothnessCertified).toBe(false)
  const preview=previewProgressiveNurbsProfiles([profile],path,scale,twist,opts,9)
  expect(preview.patches).not.toBeNull()
  expect(preview.report.closedSourceFrameSmoothness?.closedSourceFrameSmoothnessCertified).toBe(true)
 }
 const bad=structuredClone(moving);bad.values[3]![0]+=2**-54
 for(const order of [1,2])expect(inspectProgressiveClosedAuthoredFrameSmoothness([profile],path,scale,twist,{...options,frameAxis:bad},order,10000,1000000).closedSourceFrameSmoothnessCertified).toBe(order===1)
})
