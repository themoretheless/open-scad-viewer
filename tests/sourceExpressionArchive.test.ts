import {readFileSync} from 'node:fs'
import {it,expect} from 'vitest'
import {createSourceBodyRecord,withSourceBodyInspection,sourceBodyRecordInspectionOptions,isSourceBodyRecordPayload} from '../src/services/sourceBodyArchive'
import type {ExpressionInspectionOptions} from '../src/services/expressionShellInspection'
import type {SourceBodyOptions} from '../src/services/sourceBody'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-inspection-native-2026-10-09.json',import.meta.url),'utf8'))
const {op:_,...options}=fixture.request as ExpressionInspectionOptions&{op:string}
const definition=(options.proposalDefinition as {originalBody:SourceBodyOptions['definition']}).originalBody
const base=()=>createSourceBodyRecord('source','Source',{definition,limits:options.sourceLimits,endpointSpans:256})
it('retains original radius laws and work limits across archive reload',()=>{
 const radiusFixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-radius-dispatch-2026-10-09.json',import.meta.url),'utf8'))
 for(const state of ['admitted','violation','unproven']){
  const request={...structuredClone(options),radiusCheck:radiusFixture[state].request.radiusCheck}
  const original=base(),record=withSourceBodyInspection(original,request)
  const reloaded=sourceBodyRecordInspectionOptions(JSON.parse(JSON.stringify(record)))!
  expect(reloaded.radiusCheck).toEqual(request.radiusCheck)
  expect(record.source).toEqual(original.source)
  // Detached transport data must not change an already saved law.
  const radius=reloaded.radiusCheck!.radius as {controlPoints:number[][]}
  radius.controlPoints[0]![0]!+=0.25
  expect(sourceBodyRecordInspectionOptions(record)!.radiusCheck).toEqual(request.radiusCheck)
 }
})
it('persists proposed inspection recipes independently of original body admission',()=>{
 const original=base(),record=withSourceBodyInspection(original,options)
 expect(original.expressionInspection).toBeUndefined()
 expect(record.source).toEqual(original.source)
 expect(isSourceBodyRecordPayload(record)).toBe(true)
 expect(sourceBodyRecordInspectionOptions(JSON.parse(JSON.stringify(record)))).toEqual(options)
})
it('rejects an expression bound to another body and malformed archive',()=>{
 const other=structuredClone(options);(other.proposalDefinition as {originalBody:unknown}).originalBody={version:999}
 expect(()=>withSourceBodyInspection(base(),other)).toThrow('another source body')
 const broken={...base(),expressionInspection:{encoding:'MGV1/base64' as const,data:'!invalid!'}}
 expect(isSourceBodyRecordPayload(broken)).toBe(false)
 expect(()=>sourceBodyRecordInspectionOptions(broken)).toThrow()
})
it('preserves support tangency limits without changing original geometry',()=>{
 for(const spans of [0,100000]){
  const request={...structuredClone(options),supportTangencyCheck:{controls:1000,cells:10000,spans}}
  const original=base(),record=withSourceBodyInspection(original,request)
  const reloaded=sourceBodyRecordInspectionOptions(JSON.parse(JSON.stringify(record)))!
  expect(reloaded.supportTangencyCheck).toEqual(request.supportTangencyCheck)
  expect(record.source).toEqual(original.source)
  reloaded.supportTangencyCheck!.spans=17
  expect(sourceBodyRecordInspectionOptions(record)!.supportTangencyCheck).toEqual(request.supportTangencyCheck)
 }
})
it('preserves actual-neighbor angular tolerance and budgets through reload',()=>{
 for(const spans of [0,100000]){
  const request={...structuredClone(options),neighborTangencyCheck:{controls:1000,cells:10000,spans,maxSineSquared:1e-6}}
  const original=base(),record=withSourceBodyInspection(original,request)
  const reloaded=sourceBodyRecordInspectionOptions(JSON.parse(JSON.stringify(record)))!
  expect(reloaded.neighborTangencyCheck).toEqual(request.neighborTangencyCheck)
  expect(record.source).toEqual(original.source)
  reloaded.neighborTangencyCheck!.maxSineSquared=0.5
  expect(sourceBodyRecordInspectionOptions(record)!.neighborTangencyCheck).toEqual(request.neighborTangencyCheck)
 }
})

it('preserves aligned distance thresholds and work limits without saved authority',()=>{
 const request={...structuredClone(options),alignedChordCheck:{minimumMm:1.4,maxSineSquared:1e-4,toleranceUv:0.05,controls:1000,cells:10000,spans:100000,domainCells:100000}}
 const original=base(),record=withSourceBodyInspection(original,request)
 const restored=sourceBodyRecordInspectionOptions(JSON.parse(JSON.stringify(record)))!
 expect(restored.alignedChordCheck).toEqual(request.alignedChordCheck)
 expect(record.source).toEqual(original.source)
 restored.alignedChordCheck!.minimumMm=2
 restored.alignedChordCheck!.spans=0
 expect(sourceBodyRecordInspectionOptions(record)!.alignedChordCheck).toEqual(request.alignedChordCheck)
 const invalid={...request,alignedChordCheck:{...request.alignedChordCheck,minimumMm:NaN}}
 expect(()=>withSourceBodyInspection(original,invalid)).toThrow('Bound native aligned chord coverage work')
})
