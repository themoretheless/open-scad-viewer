import {nonlinearSurfaceCases} from './surface-self-fixtures'
import {SurfaceSelfIntersectionBrowser} from '../../src/services/surfaceSelfIntersectionBrowser'
import type {NurbsSurface} from '../../src/services/nurbsSurface'
function graph():NurbsSurface{return{degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],controlPoints:[0,.5,1].map(x=>[[x,0,x*x],[x,1,x*x]]),weights:[[1,1],[1,1],[1,1]],periodicU:false,periodicV:false}}
async function qualify(){
 const module=await WebAssembly.compile(await(await fetch('/__audit.wasm')).arrayBuffer())
 const lane=new SurfaceSelfIntersectionBrowser(module)
 let ticks=0;const timer=setInterval(()=>ticks++,2)
 const failure=async(p:Promise<unknown>)=>{try{await p;throw new Error('Expected refusal')}catch(e){return String(e)}}
 try{
  const surface=graph(),original=JSON.stringify(surface)
  const absence=await lane.inspect(surface,100)
  const fold=graph();fold.knotsU=[0,0,0,1,1,3,3,3];fold.controlPoints=[fold.controlPoints[0],fold.controlPoints[1],fold.controlPoints[0],fold.controlPoints[2],fold.controlPoints[2]];fold.weights=Array.from({length:5},()=>[1,1])
  const exact=await lane.inspect(fold,100)
  const controller=new AbortController(),running=lane.inspect(surface,100,{signal:controller.signal})
  const busy=await failure(lane.inspect(surface,100));controller.abort()
  const cancelled=await failure(running)
  const deadline=await failure(lane.inspect(surface,100,{deadlineMs:1}))
  const invalid=graph();invalid.weights[0][0]=-1
  const nativeRefusal=await failure(lane.inspect(invalid,100))
  const recovered=await lane.inspect(surface,100)
  const cloneFailure=await failure(lane.inspect(Object.assign(graph(),{badFunction:()=>0}),100))
  const crossing=graph();crossing.degreeU=1;crossing.knotsU=[0,0,1,2,3,3];crossing.controlPoints=[[0,0],[1,1],[0,1],[1,0]].map(([x,y])=>[[x!,y!,0],[x!,y!,2]]);crossing.weights=[1,3,2,1].map(x=>[x,x])
  const rational=await lane.inspect(crossing,100)
  const nonlinear=[]
  for(const fixture of nonlinearSurfaceCases)nonlinear.push({name:fixture.name,report:await lane.inspect(fixture.surface(),10000)})
  const preaborted=new AbortController();preaborted.abort()
  const before=lane.snapshot().workersStarted
  const preAbort=await failure(lane.inspect(surface,100,{signal:preaborted.signal}))
  const badBudget=await failure(lane.inspect(surface,0))
  return{nonlinear,absence:absence.status,exact,recovered:recovered.status,rational,
   busy,cancelled,deadline,nativeRefusal,cloneFailure,preAbort,badBudget,
   inputIntact:JSON.stringify(surface)===original,noEarlyWorkers:lane.snapshot().workersStarted===before,ticks,ownership:lane.snapshot()}
 }finally{clearInterval(timer)}
}
Object.assign(globalThis,{runSurfaceAuditQualification:qualify})
