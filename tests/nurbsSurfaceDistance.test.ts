import {expect,it} from 'vitest'
import {measureNurbsSurfaceDistance,evaluateNurbsSurface,type NurbsSurface} from '../src/services/nurbsSurface'
import {mainSolidExpectation,mainSolidResult,type MainSolidJob} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const plane=(z:number):NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,z],[0,10,z]],[[10,0,z],[10,10,z]]],weights:[[1,1],[1,1]]})
function bowl():NurbsSurface {
 const q=[.37,.62].map(x=>[x*x,x*x-x,(1-x)*(1-x)])
 return {degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],controlPoints:Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i/2,j/2,q[0][i]+q[1][j]])),weights:[[1,1,1],[1,1,1],[1,1,1]]}
}
it('finds a joint interior minimum on two curved surfaces in WASM',()=>{
 const a=bowl(),b=structuredClone(a);b.controlPoints.flat().forEach(p=>p[2]=-p[2]-2)
 const before=JSON.stringify([a,b]),r=measureNurbsSurfaceDistance(a,b,.002,100000)
 expect(r).toMatchObject({scope:'untrimmed-surfaces',converged:true,reason:'tolerance'})
 expect(r.distanceIntervalMm[0]).toBeLessThanOrEqual(2);expect(r.distanceIntervalMm[1]).toBeGreaterThanOrEqual(2)
 expect(r.distanceIntervalMm[1]-r.distanceIntervalMm[0]).toBeLessThanOrEqual(.002)
 r.parameters.forEach((uv,i)=>{expect(uv.every(t=>t>0&&t<1)).toBe(true);expect(r.points[i]).toEqual(evaluateNurbsSurface([a,b][i],...uv).point)})
 expect(JSON.stringify([a,b])).toBe(before)
})
it('retains a global interval and explicit incomplete state at the work limit',()=>{
 const a=plane(0),b=plane(2);b.controlPoints.flat().forEach(p=>{p[0]=3.7;p[1]=6.2})
 const r=measureNurbsSurfaceDistance(a,b,1e-9,1)
 expect(r).toMatchObject({converged:false,reason:'work-limit',cells:1})
 expect(r.distanceIntervalMm[0]).toBeLessThanOrEqual(2);expect(r.distanceIntervalMm[1]).toBeGreaterThanOrEqual(2)
})
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
it('includes crossings and outer boundaries of the full surface images',()=>{
 const a=plane(0),crossing=plane(0)
 crossing.controlPoints=[[[5,0,-3],[5,10,-3]],[[5,0,3],[5,10,3]]]
 const contact=measureNurbsSurfaceDistance(a,crossing)
 expect(contact.converged).toBe(true);expect(contact.distanceIntervalMm[0]).toBe(0);expect(contact.distanceIntervalMm[1]).toBeLessThanOrEqual(.001)
 const outside=plane(0);outside.controlPoints.flat().forEach(p=>{p[0]=13;p[1]=14})
 const boundary=measureNurbsSurfaceDistance(a,outside)
 expect(boundary.converged).toBe(true);expect(boundary.parameters[0]).toEqual([1,1])
 expect(boundary.distanceIntervalMm[0]).toBeLessThanOrEqual(5);expect(boundary.distanceIntervalMm[1]).toBeGreaterThanOrEqual(5)
})
