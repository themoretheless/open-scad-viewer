import {readFileSync} from 'node:fs'
import {effectScope,shallowRef,ref} from 'vue'
import {it,expect,vi} from 'vitest'
import {useExpressionInspection} from '../src/services/useExpressionInspection'
import type {ExpressionInspectionOptions,ExpressionInspectionResult} from '../src/services/expressionShellInspection'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-inspection-native-2026-10-09.json',import.meta.url),'utf8'))
const {op:_,...options}=fixture.request as ExpressionInspectionOptions&{op:string}
it('clears mesh preview on cancellation and refuses a late response with stale mesh limits',async()=>{
 const precision=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-mesh-precision-native-2026-10-09.json',import.meta.url),'utf8'))
 const request={...structuredClone(options),meshLimits:precision.request.meshLimits}
 const response=structuredClone(fixture.result)
 response.request=structuredClone(request)
 response.shell.displayMesh=precision.result.shell.displayMesh
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:()=>new Promise<ExpressionInspectionResult>(resolve=>tasks.push({resolve}))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(request),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const initial=controller.run();tasks[0].resolve(response);await initial
  expect(controller.result.value?.shell?.displayMesh?.tiles.length).toBeGreaterThan(0)
  controller.cancel();expect(controller.result.value).toBeNull()
  const late=controller.retry()
  source.value={...request,meshLimits:{...request.meshLimits,maxDeviationMm:0.025}}
  tasks[1].resolve(response);await late
  expect(controller.result.value).toBeNull()
  expect(controller.pending.value).toBe(false)
 }finally{scope.stop()}
 expect(worker.dispose).toHaveBeenCalledOnce()
})
it('clears cancelled diagnostics, ignores a late result and supports Retry',async()=>{
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void;reject:(e:Error)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn(()=>new Promise<ExpressionInspectionResult>((resolve,reject)=>tasks.push({resolve,reject})))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(structuredClone(options)),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const first=controller.run();expect(controller.pending.value).toBe(true)
  controller.cancel();tasks[0].resolve(fixture.result);await first
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(false)
  const retry=controller.retry();tasks[1].resolve(fixture.result);await retry
  expect(controller.result.value?.shell?.inspection.volumeQualified).toBe(true)
  source.value=structuredClone(options);expect(controller.result.value).toBeNull()
  const failed=controller.run();tasks[2].reject(Object.assign(new Error('Native inspection failed'),{code:'CAD_TIMEOUT'}));await failed
  expect(controller.error.value).toBe('Native inspection failed');expect(controller.errorCode.value).toBe('CAD_TIMEOUT')
  const again=controller.retry();expect(controller.errorCode.value).toBe('');active.value=false;tasks[3].resolve(fixture.result);await again
  expect(controller.result.value).toBeNull();expect(controller.error.value).toBe('')
 }finally{scope.stop()}
 expect(worker.dispose).toHaveBeenCalledOnce()
})
it('rejects a result when a raw recipe changes outside reactive notification',async()=>{
 let resolve!:(r:ExpressionInspectionResult)=>void
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:()=>new Promise<ExpressionInspectionResult>(r=>{resolve=r})}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(structuredClone(options)),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const task=controller.run();source.value!.profileLimits.controls++
  resolve(fixture.result);await task
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(false)
 }finally{scope.stop()}
})
it.each([false,true])('clears the shared boundary preview and rejects stale limits (refined=%s)',async(refined)=>{
 const refinement={maxDeviationMm:0.05,cells:512,spans:20000,spansPerQuery:64,controls:1000,domainCells:10000,domainCellsPerTriangle:128,toleranceUv:1e-8,maxDepth:8,regionalSpans:10000}
 const request={...structuredClone(options),boundaryMeshLimits:{maxDeviationMm:0.05,cells:4096,spans:100000,maxDepth:24,...(refined?{refinement}:{})}}
 const response=structuredClone(fixture.result)
 response.request=structuredClone(request)
 response.shell.boundaryMesh={version:1,materialQualified:false,surfaceToleranceQualified:false,closedOrientedIncidence:false,
  openEdges:3,nonmanifoldEdges:0,orientationConflicts:0,invalidVertexLinks:3,
  vertices:[[0,0,0],[1,0,0],[0,1,0]].map(point=>({point,bounds:point.map(x=>[x,x])})),
  segments:[],pending:[],candidateTriangles:[{face:0,vertices:[0,1,2],uv:[[0,0],[1,0],[0,1]]}],pendingFaces:[]}
 if(refined)response.shell.boundaryMesh.refinement={version:1,limits:refinement,solidQualified:false,closedOrientedIncidence:false,
  initialTriangles:1,initialChecked:1,splitTriangles:0,cells:1,spans:64,domainCells:1,regional:{spans:0,queries:0,accepted:0},
  accepted:[],pending:[{...response.shell.boundaryMesh.candidateTriangles[0],reason:'triangle-query-disabled'}],numericFailures:[]}
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:()=>new Promise<ExpressionInspectionResult>(resolve=>tasks.push({resolve}))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(request),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try {
  const initial=controller.run();tasks[0].resolve(response);await initial
  expect(controller.result.value?.shell?.boundaryMesh?.candidateTriangles.length).toBe(1)
  controller.cancel();expect(controller.result.value).toBeNull()
  const late=controller.retry()
  source.value={...request,boundaryMeshLimits:{...request.boundaryMeshLimits,...(refined?{refinement:{...refinement,regionalSpans:5000}}:{cells:2048})}}
  tasks[1].resolve(response);await late
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(false)
 }finally{scope.stop()}
})

it.each(['result','error'])('does not restart inspection after disposal and ignores a late %s',async(kind)=>{
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void;reject:(e:Error)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn(()=>new Promise<ExpressionInspectionResult>((resolve,reject)=>tasks.push({resolve,reject})))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(structuredClone(options)),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 const pending=controller.run()
 scope.stop()
 if(kind==='error')tasks[0].reject(new Error('Late worker failure after disposal'))
 else tasks[0].resolve(fixture.result)
 await pending
 const lateRetry=controller.retry()
 // Finish a wrongly launched stub request as well, so a regression cannot
 // leave an unresolved test task hiding behind the failed assertion.
 tasks[1]?.resolve(fixture.result);await lateRetry
 expect(worker.run).toHaveBeenCalledOnce()
 expect(worker.dispose).toHaveBeenCalledOnce()
 expect(controller.pending.value).toBe(false)
 expect(controller.result.value).toBeNull()
 expect(controller.error.value).toBe('')
 expect(controller.errorCode.value).toBe('')
})

it('ignores a late radius result after the original law changes',async()=>{
 const radius=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-radius-dispatch-2026-10-09.json',import.meta.url),'utf8'))
 const request={...structuredClone(options),radiusCheck:radius.admitted.request.radiusCheck}
 const response=structuredClone(fixture.result);response.request=structuredClone(request)
 response.shell.radiusCheck=radius.admitted.shell.radiusCheck
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:()=>new Promise<ExpressionInspectionResult>(resolve=>tasks.push({resolve}))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(request),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const first=controller.run()
  source.value={...structuredClone(request),radiusCheck:radius.violation.request.radiusCheck}
  const second=controller.run()
  tasks[0].resolve(response);await first
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(true)
  const current=structuredClone(fixture.result);current.request=structuredClone(source.value)
  current.shell.radiusCheck=radius.violation.shell.radiusCheck
  tasks[1].resolve(current);await second
  expect(controller.result.value?.shell?.radiusCheck?.violationQualified).toBe(true)
  expect(worker.cancel).toHaveBeenCalled()
 }finally{scope.stop()}
})
it('ignores a late support certificate after its work budget changes',async()=>{
 const native=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-support-tangency-dispatch-2026-10-09.json',import.meta.url),'utf8'))
 const request={...structuredClone(options),supportTangencyCheck:native.admitted.request.supportTangencyCheck}
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:()=>new Promise<ExpressionInspectionResult>(resolve=>tasks.push({resolve}))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(request),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const first=controller.run()
  source.value={...structuredClone(request),supportTangencyCheck:native.unproven.request.supportTangencyCheck}
  const second=controller.run()
  const old=structuredClone(fixture.result);old.request=structuredClone(request)
  old.shell.supportTangencyCheck=native.admitted.shell.supportTangencyCheck
  tasks[0].resolve(old);await first
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(true)
  const current=structuredClone(fixture.result);current.request=structuredClone(source.value)
  current.shell.supportTangencyCheck=native.unproven.shell.supportTangencyCheck
  tasks[1].resolve(current);await second
  expect(controller.result.value?.shell?.supportTangencyCheck?.qualified).toBe(false)
  expect(worker.cancel).toHaveBeenCalled()
 }finally{scope.stop()}
})

it('ignores a late actual-neighbor certificate after its work budget changes',async()=>{
 const native=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-neighbor-tangency-dispatch-2026-10-10.json',import.meta.url),'utf8'))
 const request={...structuredClone(options),neighborTangencyCheck:native.admitted.request.neighborTangencyCheck}
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:()=>new Promise<ExpressionInspectionResult>(resolve=>tasks.push({resolve}))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(request),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const first=controller.run()
  source.value={...structuredClone(request),neighborTangencyCheck:native.unproven.request.neighborTangencyCheck}
  const second=controller.run()
  const old=structuredClone(fixture.result);old.request=structuredClone(request)
  old.shell.neighborTangencyCheck=native.admitted.shell.neighborTangencyCheck
  tasks[0].resolve(old);await first
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(true)
  const current=structuredClone(fixture.result);current.request=structuredClone(source.value)
  current.shell.neighborTangencyCheck=native.unproven.shell.neighborTangencyCheck
  tasks[1].resolve(current);await second
  expect(controller.result.value?.shell?.neighborTangencyCheck?.qualified).toBe(false)
  expect(worker.cancel).toHaveBeenCalled()
 }finally{scope.stop()}
})
it('cancels and ignores actual-neighbor authority when only angular tolerance changes',async()=>{
 const native=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-neighbor-tangency-dispatch-2026-10-10.json',import.meta.url),'utf8'))
 const request={...structuredClone(options),neighborTangencyCheck:native.admitted.request.neighborTangencyCheck}
 let deliver!:(v:ExpressionInspectionResult)=>void
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:()=>new Promise<ExpressionInspectionResult>(resolve=>{deliver=resolve})}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(request),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const first=controller.run()
  source.value={...structuredClone(request),neighborTangencyCheck:{...request.neighborTangencyCheck,maxSineSquared:1e-7}}
  const old=structuredClone(fixture.result);old.request=structuredClone(request)
  old.shell.neighborTangencyCheck=native.admitted.shell.neighborTangencyCheck
  deliver(old);await first
  expect(controller.result.value).toBeNull()
  expect(worker.cancel).toHaveBeenCalled()
 }finally{scope.stop()}
})

it.each(['minimumMm','maxSineSquared','toleranceUv','controls','cells','spans','domainCells'] as const)('rejects late aligned-chord diagnostics after %s changes and retries current parameters',async(field)=>{
 const request={...structuredClone(options),alignedChordCheck:{minimumMm:1,maxSineSquared:1e-4,toleranceUv:0.05,controls:1000,cells:0,spans:0,domainCells:0}}
 const responseFor=(current:typeof request)=>{
  const response=structuredClone(fixture.result);response.request=structuredClone(current)
  const faces=response.shell.faceCount,pending=[]
  for(let a=0;a<faces;a++)for(let b=a;b<faces;b++)pending.push({faces:[a,b],domains:[[[0,1],[0,1]],[[0,1],[0,1]]],pending_reason:'budget'})
  const report:any={method:'expression-support-aligned-chord-coverage',scope:'original-retained-transition-support-products',minimum_mm:current.alignedChordCheck.minimumMm,max_sine_squared:current.alignedChordCheck.maxSineSquared,tolerance_uv:current.alignedChordCheck.toleranceUv,complete:false,total_pairs:pending.length,pending,budget_pending:pending.length}
  for(const key of ['cells','spans','domain_cells','initial_pairs_checked','intrinsic_checked','intrinsic_spans','intrinsic_unresolved','profile_transition_checked','profile_transition_excluded','profile_transition_spans','distance_excluded','angle_excluded','planar_excluded','intrinsic_excluded','plane_controls','cache_hits','symmetry_reused','trim_excluded','enclosure_pending','resolution_pending'])report[key]=0
  response.shell.alignedChordCheck={version:1,qualified:false,wholeThicknessQualified:false,solidQualified:false,report};return response
 }
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:()=>new Promise<ExpressionInspectionResult>(resolve=>tasks.push({resolve}))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(request),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const first=controller.run();tasks[0].resolve(responseFor(request));await first
  expect(controller.result.value?.shell?.alignedChordCheck?.qualified).toBe(false)
  const stale=controller.run()
  const next={...structuredClone(request),alignedChordCheck:{...request.alignedChordCheck,[field]:field==='maxSineSquared'?2e-4:field==='toleranceUv'?0.1:request.alignedChordCheck[field]+1}}
  source.value=next;expect(controller.result.value).toBeNull()
  const latest=controller.retry();tasks[1].resolve(responseFor(request));await stale
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(true)
  tasks[2].resolve(responseFor(next));await latest
  expect(controller.result.value?.request.alignedChordCheck).toEqual(next.alignedChordCheck)
  expect(worker.cancel).toHaveBeenCalled()
 }finally{scope.stop()}
})

it('classifies invalid aligned parameters before dispatch and clears the code on a corrected retry',async()=>{
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn(async()=>fixture.result)}
 const source=shallowRef<ExpressionInspectionOptions|undefined>({...structuredClone(options),
  alignedChordCheck:{minimumMm:NaN,maxSineSquared:1e-4,toleranceUv:0.05,controls:1000,cells:10000,spans:100000,domainCells:100000}})
 const scope=effectScope(),active=ref(true)
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  await controller.run()
  expect(controller.errorCode.value).toBe('CAD_REQUEST')
  expect(controller.error.value).not.toBe('');expect(worker.run).not.toHaveBeenCalled()
  source.value=structuredClone(options)
  expect(controller.errorCode.value).toBe('');expect(controller.error.value).toBe('')
  await controller.retry()
  expect(worker.run).toHaveBeenCalledOnce();expect(controller.errorCode.value).toBe('')
  expect(controller.result.value).toEqual(fixture.result)
 }finally{scope.stop()}
})

it.each(['valid','invalid'])('ignores a late worker error after a raw %s recipe mutation and allows Retry',async(kind)=>{
 const tasks:{resolve:(v:ExpressionInspectionResult)=>void;reject:(e:Error)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn(()=>new Promise<ExpressionInspectionResult>((resolve,reject)=>tasks.push({resolve,reject})))}
 const source=shallowRef<ExpressionInspectionOptions|undefined>(structuredClone(options)),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useExpressionInspection(source,active,worker as never))!
 try{
  const first=controller.run()
  source.value!.profileLimits.controls=kind==='invalid'?NaN:source.value!.profileLimits.controls+1
  tasks[0].reject(Object.assign(new Error('Failure from the old recipe'),{code:'CAD_TIMEOUT'}));await first
  expect(controller.error.value).toBe('');expect(controller.errorCode.value).toBe('')
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(false)
  source.value=structuredClone(options)
  const retry=controller.retry();tasks[1].resolve(fixture.result);await retry
  expect(controller.result.value).toEqual(fixture.result)
  expect(controller.error.value).toBe('');expect(worker.run).toHaveBeenCalledTimes(2)
 }finally{scope.stop()}
})
