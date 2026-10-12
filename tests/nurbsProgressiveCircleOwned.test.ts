import {expect,it} from 'vitest'
import {circleNurbsCurve,bezierNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'

it('owns explicit progressive profile correction against the original hollow source and filled caps',()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
 const source=JSON.stringify(loops)
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const options={normal:[1,0,0] as [number,number,number],orientation:'fixed' as const,initialSections:3,maxSections:3,maxDeviation:.01,
  circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:1000000},capCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:1000000}}
 const construct=(maxWork:number)=>createProgressiveBrepProfileBody(loops,bezierNurbsCurve([[0,0,0],[0,0,2]]),law,{...law,values:[0,0]},
  {...options,circleCorrection:{...options.circleCorrection,maxWork}})
 const body=construct(1000000)
 expect(JSON.stringify(loops)).toBe(source)
 expect(body).toMatchObject({boundaryContinuousBound:true,boundaryErrorWithinBudget:true,globalEmbeddingCertified:false,
  retainedWalls:{certified:true},retainedCaps:{exact:true}})
 expect(body.profileCorrectionErrorUpper).toBeGreaterThan(0)
 expect(body.profileCorrectionErrorUpper).toBeLessThanOrEqual(1e-9)
 expect(body.filledCapErrorUpper!.every(e=>e>=body.profileCorrectionErrorUpper!)).toBe(true)
 const caps=[body.model.faces.length-2,body.model.faces.length-1]
 const actual=inspectMiterProfileSmoothness(body.model,caps)
 expect(actual.profileG1Certified).toBe(true)
 expect(actual.profile.exactG1G2Certified).toBe(true)
 expect(()=>construct(0)).toThrow(/profile circle correction refused/i)
})
