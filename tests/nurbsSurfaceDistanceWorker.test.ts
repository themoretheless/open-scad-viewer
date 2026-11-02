import {expect,it} from 'vitest'
import {measureNurbsSurfaceDistance,type NurbsSurface} from '../src/services/nurbsSurface'
import {mainSolidExpectation,mainSolidResult,type MainSolidJob} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const plane=(z:number):NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,z],[0,10,z]],[[10,0,z],[10,10,z]]],weights:[[1,1],[1,1]]})
it('rejects surface worker results with wrong domains, scope, budgets or completion claims',()=>{
 const job:MainSolidJob={kind:'surfaceDistance',a:plane(0),b:plane(3),toleranceMm:.001,maxCells:100}
 const r=measureNurbsSurfaceDistance(job.a,job.b,job.toleranceMm,job.maxCells),expectation=mainSolidExpectation(job)
 expect(mainSolidResult(expectation,r)).toBe(true)
 for(const patch of [{scope:'trimmed-faces'},{maxCells:101},{toleranceMm:.1},{cells:101},{parameters:[[2,0],[0,0]]},{parameters:[0,0]},{points:[[0,0],[0,3]]},{distanceIntervalMm:[4,3]},{distanceIntervalMm:[0,3]},{reason:'work-limit'},{converged:false},{distanceIntervalMm:[NaN,3]}])expect(mainSolidResult(expectation,{...r,...patch})).toBe(false)
})
it('dispatches complete surface distance and recovers from invalid input in the worker',async()=>{
 const messages:any[]=[],handler=createMainSolidWorkerHandler(message=>messages.push(message))
 const job={kind:'surfaceDistance' as const,a:plane(0),b:plane(3),toleranceMm:.001,maxCells:100}
 await handler({version:1,id:1,job});expect(messages[0]).toMatchObject({id:1,ok:true,result:{scope:'untrimmed-surfaces',converged:true}})
 await handler({version:1,id:2,job:{...job,toleranceMm:0}});expect(messages[1]).toMatchObject({id:2,ok:false,error:{code:'NURBS_INVALID_INPUT'}})
 await handler({version:1,id:3,job});expect(messages[2]).toMatchObject({id:3,ok:true})
})
