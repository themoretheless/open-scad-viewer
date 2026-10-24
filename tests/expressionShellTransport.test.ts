import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import {readFileSync} from 'node:fs'
import {describe,it,expect} from 'vitest'
import {expressionShellExpectation,validExpressionShell,type ExpressionShellOptions} from '../src/services/expressionShell'
import {mainSolidExpectation,mainSolidResult,MAIN_SOLID_JOB_KINDS} from '../src/services/mainSolidProtocol'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-replay-request-2026-10-09.json',import.meta.url),'utf8'))
const {op:_,...options}=fixture as ExpressionShellOptions&{op:string}
const refusal=()=>({version:1,method:'exact-expression-shell-replay',request:structuredClone(options),proposalDefinition:options.proposalDefinition,boundaryDefinition:options.boundaryDefinition,incidenceQualified:false,embeddingQualified:false,materialQualified:false,reason:'profile-work-limit',shell:null})
describe('expression shell transport identity',()=>{
 it('routes native refusal without promoting it to a solid',()=>{
  expect(MAIN_SOLID_JOB_KINDS.expressionShellRestore).toBe(true)
  const job=mainSolidExpectation({kind:'expressionShellRestore',options})
  expect(mainSolidResult(job,refusal())).toBe(true)
  expect(mainSolidResult(job,{...refusal(),materialQualified:true})).toBe(false)
  expect(mainSolidResult(job,{...refusal(),embeddingQualified:true})).toBe(false)
 })
 it('rejects replies for another budget or source recipe',()=>{
  const expected=expressionShellExpectation(options)
  const other=refusal();other.request.profileLimits.controls++
  expect(validExpressionShell(expected,other)).toBe(false)
  expect(validExpressionShell(expected,{...refusal(),boundaryDefinition:{version:1}})).toBe(false)
 })
 it('distinguishes objects from arrays in recipe identity',()=>{
  const original={...options,boundaryDefinition:{a:1}}
  const expected=expressionShellExpectation(original)
  const changed={...original,boundaryDefinition:[['a',1]]}
  expect(validExpressionShell(expected,{...refusal(),request:changed,proposalDefinition:original.proposalDefinition,boundaryDefinition:changed.boundaryDefinition})).toBe(false)
 })
 it('rejects oversized topology metadata before preview',()=>{
  const expected=expressionShellExpectation(options)
  const shell={faces:[{id:0,kind:'original',source:0,loops:1}],faceCount:1,pairCount:1,vertexCount:2,poleCount:0,components:1}
  const result={...refusal(),incidenceQualified:true,shell}
  expect(validExpressionShell(expected,result)).toBe(true)
  expect(validExpressionShell(expected,{...result,shell:{...shell,vertexCount:options.shellLimits.shellUses*2+1}})).toBe(false)
  expect(validExpressionShell(expected,{...result,shell:{...shell,faces:[{...shell.faces[0],loops:options.shellLimits.shellUses+1}]}})).toBe(false)
 })
 it('captures request identity before caller mutation' ,()=>{
  const mutable=structuredClone(options);const expected=expressionShellExpectation(mutable)
  mutable.shellLimits.shellUses++
  expect(validExpressionShell(expected,{...refusal(),request:mutable})).toBe(false)
 })
 it('binds optional display work and rejects missing or unsolicited display replies',()=>{
  const displayLimits={divisions:1,cells:1,controlsPerEnclosure:1000,spans:100,toleranceUv:1e-8,domainCells:100,domainCellsPerCell:16}
  const request={...options,displayLimits}
  const shell={faces:[{id:0,kind:'original',source:0,loops:1}],faceCount:1,pairCount:1,vertexCount:2,poleCount:0,components:1}
  const supportEnclosures={trimQualified:false,enclosed:1,pending:0,spans:1,domainCells:1,inside:1,outside:0,unresolvedTrim:0,
   cells:[{face:0,domain:[[0,1],[0,1]],bounds:[[0,1],[0,1],[0,1]],location:'inside'}]}
  const result={...refusal(),request,incidenceQualified:true,shell:{...shell,supportEnclosures}}
  const expected=expressionShellExpectation(request)
  expect(validExpressionShell(expected,result)).toBe(true)
  expect(validExpressionShell(expected,{...result,shell})).toBe(false)
  expect(validExpressionShell(expressionShellExpectation(options),{...result,request:options})).toBe(false)
  displayLimits.domainCells++
  expect(validExpressionShell(expected,result)).toBe(false)
 })
})

it('cancels expression replay and ignores a late reply during Retry',async()=>{
 const ports:(MainSolidPort&{request?:import('../src/services/mainSolidProtocol').MainSolidRequest;terminated:boolean})[]=[]
 const client=new MainSolidWorkerClient(()=>{
  const port:typeof ports[number]={onmessage:null,onerror:null,onmessageerror:null,terminated:false,
   postMessage(request){this.request=request},terminate(){this.terminated=true}}
  ports.push(port);return port
 })
 try{
  const first=client.run({kind:'expressionShellRestore',options})
  const rejected=expect(first).rejects.toMatchObject({code:'CAD_CANCELLED'})
  const late=ports[0].onmessage;const old=ports[0].request!
  client.cancel();await rejected;expect(ports[0].terminated).toBe(true)
  const retry=client.run({kind:'expressionShellRestore',options})
  late?.({data:{version:1,id:old.id,kind:old.job.kind,ok:true,result:{...refusal(),materialQualified:true}}} as MessageEvent)
  const fresh=ports[1].request!
  ports[1].onmessage?.({data:{version:1,id:fresh.id,kind:fresh.job.kind,ok:true,result:refusal()}} as MessageEvent)
  await expect(retry).resolves.toMatchObject({incidenceQualified:false,materialQualified:false})
 }finally{client.dispose()}
})
