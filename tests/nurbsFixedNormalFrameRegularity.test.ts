import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {inspectFixedNormalFrameRegularity} from '../src/services/nurbsFixedNormalFrameRegularity'

const zero=()=>bezierNurbsCurve([[0,0,0],[0,0,0]])

it('carries a complete original FixedNormal frame cover without promoting body or surface proofs',()=>{
 const path=circleNurbsCurve([0,0,0],[0,0,1],5)
 const r=inspectFixedNormalFrameRegularity(path,[0,0,1],zero())
 expect(r).toMatchObject({regularityCertified:true,continuousBound:false,surfaceRegularityCertified:false,globalEmbeddingCertified:false,
  method:'original-path-adaptive-fixed-normal-frame-regularity'})
 expect(r.certifiedIntervals).toBeGreaterThan(1)
 expect(r.cells).toBeLessThanOrEqual(10000)
 const short=inspectFixedNormalFrameRegularity(path,[0,0,1],zero(),r.cells-1)
 expect(short.regularityCertified).toBe(false)
 const empty=inspectFixedNormalFrameRegularity(path,[0,0,1],zero(),0)
 expect(empty).toMatchObject({regularityCertified:false,cells:0})
})

it('refuses an interior FixedNormal singularity despite regular sampled endpoints',()=>{
 const path=bezierNurbsCurve([[0,0,0],[-0.5,0,0.5],[0,0,1]])
 const r=inspectFixedNormalFrameRegularity(path,[0,0,1],zero(),1000)
 expect(r.regularityCertified).toBe(false)
 expect(r.cells).toBeLessThanOrEqual(1000)
 expect(r.continuousBound).toBe(false)
})
