import {expect,it} from 'vitest'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {DEFAULT_SWEEP_VOLUME_BUDGETS,inspectSweepVolume} from '../src/services/nurbsSweepEmbedding'

it('refuses a globally folded closed Fixed-frame body despite a certified boundary error',()=>{
 // Fixed parallel section planes on a closed translating path do not produce
 // a globally embedded tube. Local/source error evidence must not admit Solid.
 const profile=circleNurbsCurve([0,0,0],[0,1,0],.25)
 const path=circleNurbsCurve([0,0,0],[0,0,1],10)
 const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const body=createProgressiveBrepProfileBody([[profile]],path,law(1),law(0),{
  orientation:'fixed',normal:[0,0,1],spacing:'arc_length',lengthTolerance:.001,
  initialSections:5,maxSections:129,maxDeviation:.02,
 })
 expect(body.approximation.report).toMatchObject({accepted:true,closedPath:true,continuousBound:true})
 const volume=inspectSweepVolume(body.model,[],DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(volume.solidGeometryCertified).toBe(false)
 expect(volume.boundaryEmbeddingCertified).toBe(false)
})
