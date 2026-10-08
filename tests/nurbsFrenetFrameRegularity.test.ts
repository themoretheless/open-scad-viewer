import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {inspectFrenetFrameRegularity} from '../src/services/nurbsFrenetFrameRegularity'

const zero=()=>({...bezierNurbsCurve([[0,0,0],[0,0,0]]),knots:[7,7,9,9]})
it('covers the original spatial Frenet frame on an independent authored domain',()=>{
 const path={...bezierNurbsCurve([[0,0,0],[1/3,0,0],[2/3,1/3,0],[1,1,1]]),knots:[2,2,2,2,5,5,5,5]}
 const r=inspectFrenetFrameRegularity(path,zero())
 expect(r).toMatchObject({regularityCertified:true,continuousBound:false,surfaceRegularityCertified:false,globalEmbeddingCertified:false,
  method:'original-path-adaptive-frenet-frame-regularity'})
 expect(r.certifiedIntervals).toBeGreaterThan(0)
 expect(r.cells).toBeGreaterThan(0)
 expect(inspectFrenetFrameRegularity(path,zero(),r.cells-1).regularityCertified).toBe(false)
 expect(inspectFrenetFrameRegularity(path,zero(),0)).toMatchObject({regularityCertified:false,cells:0})
})
it('refuses an interior inflection despite regular endpoints',()=>{
 const path=bezierNurbsCurve([[-1,-1,0],[-1/3,1,0],[1/3,-1,0],[1,1,0]])
 const r=inspectFrenetFrameRegularity(path,zero(),1000)
 expect(r.regularityCertified).toBe(false)
 expect(r.cells).toBeLessThanOrEqual(1000)
 expect(r.continuousBound).toBe(false)
})

it('covers the whole rational closed path without a seam smoothness claim',()=>{
 const path=circleNurbsCurve([0,0,0],[0,0,1],5)
 const r=inspectFrenetFrameRegularity(path,zero())
 expect(r).toMatchObject({regularityCertified:true,continuousBound:false,surfaceRegularityCertified:false,globalEmbeddingCertified:false})
 expect(r.certifiedIntervals).toBeGreaterThan(1)
 expect(inspectFrenetFrameRegularity(path,zero(),r.cells-1).regularityCertified).toBe(false)
})
