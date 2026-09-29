import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {measureFaceDistance,type FaceDistanceOptions} from '../src/services/solidMeasurements'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const fixture=JSON.parse(readFileSync(new URL('./fixtures/face-distance.json',import.meta.url),'utf8'))
const options=():FaceDistanceOptions=>({a:structuredClone(fixture.document.bodies[0].brep),b:structuredClone(fixture.document.bodies[1].brep),faceA:fixture.faceA,faceB:fixture.faceB,toleranceMm:.001,toleranceUv:1e-7,maxCells:100000,maxDomainCells:1000000})
it('measures the trimmed face rather than the missing material inside its hole',()=>{
 const o=options(),before=JSON.stringify(o),r=measureFaceDistance(o)
 expect(r.converged).toBe(true)
 expect(r.distanceIntervalMm[0]).toBeLessThanOrEqual(fixture.expectedMm)
 expect(r.distanceIntervalMm[1]).toBeGreaterThanOrEqual(fixture.expectedMm)
 expect(r.distanceIntervalMm[1]!-r.distanceIntervalMm[0]).toBeLessThanOrEqual(.001)
 const [a,b]=r.points!
 expect(a[0]<=4||a[0]>=6||a[1]<=4||a[1]>=6).toBe(true)
 expect(a[2]).toBeCloseTo(0,10);expect(b[2]).toBeCloseTo(2,10)
 expect(JSON.stringify(o)).toBe(before)
})
it('reports missing witnesses honestly when domain work runs out',()=>{
 const o={...options(),maxDomainCells:1},r=measureFaceDistance(o)
 expect(r).toMatchObject({converged:false,reason:'domain-work-limit',points:null,parameters:null,pointEnclosures:null})
 expect(r.distanceIntervalMm[1]).toBeNull()
 expect(mainSolidResult(mainSolidExpectation({kind:'faceDistance',options:o}),r)).toBe(true)
})
it('rejects malformed, mismatched and falsely completed face results',()=>{
 const o=options(),r=measureFaceDistance(o),e=mainSolidExpectation({kind:'faceDistance',options:o})
 expect(mainSolidResult(e,r)).toBe(true)
 for(const patch of [{scope:'untrimmed-surfaces'},{maxDomainCells:2},{toleranceUv:.1},{distanceIntervalMm:[0,null]},{parameters:[[1e9,0],[0,0]]},{reason:'work-limit'},{points:null},{domainCells:1000001}])expect(mainSolidResult(e,{...r,...patch})).toBe(false)
})
it('recovers from invalid face input in the worker',async()=>{
 const messages:any[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(m)),o=options()
 await handler({version:1,id:1,job:{kind:'faceDistance',options:{...o,faceA:99999}}})
 expect(messages[0]).toMatchObject({id:1,ok:false})
 await handler({version:1,id:2,job:{kind:'faceDistance',options:o}})
 expect(messages[1]).toMatchObject({id:2,ok:true,result:{converged:true}})
})
