import {readFileSync} from 'node:fs'
import {expect,it,vi} from 'vitest'
import {profileBridgeChordExpectation,validProfileBridgeChordReport,type ProfileBridgeChordOptions} from '../src/services/nurbsProfileBridgeChords'
import {mainSolidExpectation,mainSolidResult,MAIN_SOLID_JOB_KINDS,type MainSolidRequest} from '../src/services/mainSolidProtocol'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/profile-bridge-native-transport-2026-10-10.json','utf8'))
const {op:_,...options}=fixture.request as ProfileBridgeChordOptions&{op:string}
it('binds the actual native report to both profiles, parameters and work limits',()=>{
 const expected=profileBridgeChordExpectation(options)
 expect(validProfileBridgeChordReport(expected,fixture.report)).toBe(true)
 expect(MAIN_SOLID_JOB_KINDS.profileBridgeChords).toBe(true)
 expect(mainSolidResult(mainSolidExpectation({kind:'profileBridgeChords',options}),fixture.report)).toBe(true)
 for(const field of ['surfaces','pair','parameters','scales','pivots','axes','maxSpans','maxControls','maxSineSquared']){
  const stale=structuredClone(fixture.report)
  if(field==='surfaces')stale.request.surfaces[0].controlPoints[0][0][2]+=1
  else if(field==='pair')stale.request.pair[0][0][0]+=0.001
  else if(['parameters','scales','axes'].includes(field))stale.request[field][0]+=1
  else if(field==='pivots')stale.request.pivots[0][0]+=1
  else stale.request[field]+=1
  expect(validProfileBridgeChordReport(expected,stale),field).toBe(false)
 }
})
it('rejects forged admission flags, incomplete bounds and impossible work',()=>{
 const expected=profileBridgeChordExpectation(options)
 for(const patch of [{solidQualified:true},{wholeThicknessQualified:true},{proposedExpression:false},
  {spans:options.maxSpans+1},{profileControls:options.maxControls+1},{excluded:'true'},
  {reason:'profile-bridge-chord-unproven'},{profilePositionDeviationUpperMm:undefined},
  {profilePositionDeviationUpperMm:[[0,0,-1],[0,0,0]]},{profileWeightDeviationUpper:[0,NaN]}])
  expect(validProfileBridgeChordReport(expected,{...fixture.report,...patch})).toBe(false)
 const zero={...options,maxSpans:0},request={op:'profile_section_bridge_chords',...zero}
 expect(validProfileBridgeChordReport(profileBridgeChordExpectation(zero),{...fixture.report,request,spans:0})).toBe(false)
 expect(validProfileBridgeChordReport(profileBridgeChordExpectation(zero),{...fixture.report,request,spans:0,excluded:null,reason:'profile-bridge-chord-unproven'})).toBe(true)
 for(const maxSineSquared of [NaN,Infinity,-1,1])expect(()=>profileBridgeChordExpectation({...options,maxSineSquared})).toThrow()
})
class Port implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null
 onerror:MainSolidPort['onerror']=null
 onmessageerror:MainSolidPort['onmessageerror']=null
 terminate=vi.fn()
 postMessage=vi.fn<(request:MainSolidRequest)=>void>()
 reply(value:object){const {version,id,job}=this.postMessage.mock.lastCall![0];this.onmessage?.({data:{version,id,kind:job.kind,...value}} as MessageEvent)}
}
it('cancels this typed job, ignores a terminated late reply and admits fresh Retry',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p})
 try{
  const first=client.run({kind:'profileBridgeChords',options}),rejected=expect(first).rejects.toMatchObject({name:'AbortError'})
  const old=ports[0].onmessage!,request=ports[0].postMessage.mock.lastCall![0]
  client.cancel();await rejected;expect(ports[0].terminate).toHaveBeenCalledOnce()
  const retry=client.run({kind:'profileBridgeChords',options})
  old({data:{version:request.version,id:request.id,kind:request.job.kind,ok:true,result:fixture.report}} as MessageEvent)
  ports[1].reply({ok:true,result:fixture.report})
  await expect(retry).resolves.toEqual(fixture.report)
 }finally{client.dispose()}
})

it('keeps native profile modification bounds visible for nonseparable rational sources',()=>{
 const {op:_,...rationalOptions}=fixture.rationalProposed.request as ProfileBridgeChordOptions&{op:string}
 const report=fixture.rationalProposed.report
 expect(validProfileBridgeChordReport(profileBridgeChordExpectation(rationalOptions),report)).toBe(true)
 expect(report.profileWeightDeviationUpper[1]).toBeGreaterThanOrEqual(1)
 expect(report.profilePositionDeviationUpperMm[1][2]).toBeGreaterThan(0)
 expect(report.proposedExpression).toBe(true)
 expect(report.solidQualified).toBe(false)
 expect(report.wholeThicknessQualified).toBe(false)
})

it('preserves the native unresolved rational normal diameter',()=>{
 const {op:_,...diameterOptions}=fixture.normalDiameter.request as ProfileBridgeChordOptions&{op:string}
 const report=fixture.normalDiameter.report
 expect(validProfileBridgeChordReport(profileBridgeChordExpectation(diameterOptions),report)).toBe(true)
 expect(report.excluded).not.toBe(true)
 expect(report.solidQualified).toBe(false)
})
it('validates whole-product coverage separately from the local pair result',()=>{
 for(const key of ['wholeCoverage','stoppedCoverage']){
  const {op:_,...input}=fixture[key].request as ProfileBridgeChordOptions&{op:string}
  const report=fixture[key].report,expected=profileBridgeChordExpectation(input)
  expect(validProfileBridgeChordReport(expected,report)).toBe(true)
  for(const patch of [{cells:input.coverage!.cells+1},{spans:input.coverage!.spans+1},
   {exclusions:report.coverage.cells+1},{unresolvedPairs:[] ,completeExclusion:false},
   {unresolvedPairs:[[[[-1,0],[0,1]],[[0,1],[0,1]]]],completeExclusion:false}])
   expect(validProfileBridgeChordReport(expected,{...report,coverage:{...report.coverage,...patch}})).toBe(false)
  expect(validProfileBridgeChordReport(expected,{...report,coverage:undefined})).toBe(false)
 }
 const {op:_,...stopped}=fixture.stoppedCoverage.request as ProfileBridgeChordOptions&{op:string}
 const report=fixture.stoppedCoverage.report
 expect(report.excluded).toBe(true)
 expect(report.coverage.completeExclusion).toBe(false)
 expect(validProfileBridgeChordReport(profileBridgeChordExpectation(stopped),{...report,
  coverage:{...report.coverage,unresolvedPairs:[],completeExclusion:true}})).toBe(false)
})
