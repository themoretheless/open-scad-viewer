import {expect,it} from 'vitest'
import {measureNurbsCurveDistance,evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {mainSolidExpectation,mainSolidResult,type MainSolidJob} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const line=(points:number[][]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:points,weights:[1,1]})
const arc:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[10,0,0],[10,10,0],[0,10,0]],weights:[1,Math.SQRT1_2,1]}
it('measures rational curves in WASM, preserving original parameters and the document',()=>{
 const b=structuredClone(arc);b.controlPoints.forEach(p=>p[0]+=30)
 const before=JSON.stringify([arc,b]),result=measureNurbsCurveDistance(arc,b)
 const exact=Math.sqrt(1000)-10
 expect(result.converged).toBe(true)
 expect(result.distanceIntervalMm[0]).toBeLessThanOrEqual(exact)
 expect(result.distanceIntervalMm[1]).toBeGreaterThanOrEqual(exact)
 expect(result.distanceIntervalMm[1]-result.distanceIntervalMm[0]).toBeLessThanOrEqual(.001)
 result.points.forEach((p,i)=>expect(p).toEqual(evaluateNurbsCurve([arc,b][i],result.parameters[i]).point))
 expect(JSON.stringify([arc,b])).toBe(before)
})
it('returns unresolved global bounds rather than a success on an exhausted budget',()=>{
 const d=Math.SQRT1_2,a=line([[0,0],[10,10]]),b=line([[-d,d],[10-d,10+d]])
 const result=measureNurbsCurveDistance(a,b,1e-9,1)
 expect(result).toMatchObject({converged:false,reason:'work-limit',cells:1,maxCells:1})
 expect(result.distanceIntervalMm[0]).toBeLessThanOrEqual(1)
 expect(result.distanceIntervalMm[1]).toBeGreaterThanOrEqual(1)
})
it('admits only results matching dimensions, parameter domains, tolerance and budget',()=>{
 const job:MainSolidJob={kind:'curveDistance',a:line([[0,0],[1,0]]),b:line([[0,3],[1,3]]),toleranceMm:.001,maxCells:100}
 const expectation=mainSolidExpectation(job),result=measureNurbsCurveDistance(job.a,job.b,job.toleranceMm,job.maxCells)
 expect(mainSolidResult(expectation,result)).toBe(true)
 for(const patch of [{maxCells:101},{toleranceMm:.1},{cells:101},{parameters:[2,0]},{points:[[0,0,0],[0,3,0]]},{pointEnclosures:[[[0,1],[2,1]],[[0,1],[3,3]]]},{distanceIntervalMm:[4,3]},{distanceIntervalMm:[0,3]},{reason:'work-limit'},{converged:false},{distanceIntervalMm:[NaN,3]}])expect(mainSolidResult(expectation,{...result,...patch})).toBe(false)
})
it('runs both successful and invalid distance requests through the worker and recovers',async()=>{
 const messages:any[]=[],handler=createMainSolidWorkerHandler(message=>messages.push(message))
 const job={kind:'curveDistance' as const,a:line([[0,0],[1,0]]),b:line([[0,3],[1,3]]),toleranceMm:.001,maxCells:100}
 await handler({version:1,id:1,job})
 expect(messages[0]).toMatchObject({id:1,kind:'curveDistance',ok:true,result:{converged:true}})
 await handler({version:1,id:2,job:{...job,toleranceMm:0}})
 expect(messages[1]).toMatchObject({id:2,ok:false,error:{code:'NURBS_INVALID_INPUT'}})
 await handler({version:1,id:3,job})
 expect(messages[2]).toMatchObject({id:3,ok:true})
})
