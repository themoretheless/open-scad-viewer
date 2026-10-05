import {beforeAll,expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {fitNurbsOffsetEnvelope,envelopeFitExpectation,validEnvelopeFit} from '../src/services/nurbsOffsetEnvelopeFit'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {planeEnvelopeFixture} from './fixtures/offset-envelope'
import {curvedTangentFixture} from './fixtures/offset-contact-tangent'
beforeAll(async()=>{await warmGeometryKernel()})
const planeOptions=()=>({...planeEnvelopeFixture(),toleranceMm:1e-3,maxCells:511})
it('returns the finite rational candidate with a complete pointwise qualification from actual WASM',()=>{
 const options=planeOptions(),before=structuredClone(options),r=fitNurbsOffsetEnvelope(options)
 expect(r).toMatchObject({candidateSource:'section-proposal',proposalSections:2,wholeCurveComplete:false,tangentToleranceProven:false,trimMembershipProven:false,embeddingProven:false,topologyAuthority:false})
 expect(r.qualification).toMatchObject({approximationWithinToleranceProven:true,finiteNurbsPatchProven:true})
 expect(mainSolidResult(mainSolidExpectation({kind:'offsetEnvelopeFit',options}),r)).toBe(true)
 const q=r.qualification!;expect(q.partition).toHaveLength(q.visitedCells);expect(q.visitedCells).toBe(2*q.cells.length-1)
 expect(q.cells.every(c=>c.errorUpperMm!==null&&c.errorUpperMm<=options.toleranceMm)).toBe(true)
 for(const t of [.35,.37,.39])for(const s of [0,.25,.5,.75,1]){
  const p=evaluateNurbsSurface(r.candidateSurface!,t,s).point
  expect(Math.abs(Math.hypot(p[1]-.3,p[2]-.2)-.2)).toBeLessThanOrEqual(options.toleranceMm)
 }
 expect(options).toEqual(before)
})
it('qualifies an authored candidate, binds its snapshot, and retains work stops and mismatches',()=>{
 const options=planeOptions(),proposed=fitNurbsOffsetEnvelope(options),candidate=proposed.candidateSurface!
 const query={...options,candidate},expected=envelopeFitExpectation(query),r=fitNurbsOffsetEnvelope(query)
 expect(r).toMatchObject({candidateSource:'authored',proposalSections:0});expect(validEnvelopeFit(expected,r)).toBe(true)
 const changed=structuredClone(r);changed.candidateSurface!.controlPoints[0][0][2]+=.01;expect(validEnvelopeFit(expected,changed)).toBe(false)
 const limitedQuery={...query,maxCells:1},limited=fitNurbsOffsetEnvelope(limitedQuery)
 expect(limited.qualification).toMatchObject({finiteNurbsPatchProven:false,visitedCells:1,reason:'fit-work-limit'})
 expect(limited.qualification!.cells[0].domain).toEqual([[.35,.39],[0,1]])
 expect(validEnvelopeFit(envelopeFitExpectation(limitedQuery),limited)).toBe(true)
 const corrupt=structuredClone(candidate);for(const row of corrupt.controlPoints)for(const p of row)p[2]+=.01
 const wrong={...query,candidate:corrupt},mismatch=fitNurbsOffsetEnvelope(wrong)
 expect(mismatch.qualification).toMatchObject({finiteNurbsPatchProven:false,reason:'candidate-mismatch'})
 expect(mismatch.qualification!.cells.some(c=>c.anchorErrorIntervalMm!==null&&c.anchorErrorIntervalMm[0]>options.toleranceMm)).toBe(true)
 expect(validEnvelopeFit(envelopeFitExpectation(wrong),mismatch)).toBe(true)
})
it('rejects missing leaves, cycles, omitted tree nodes, changed bounds and promoted gates',()=>{
 const options=planeOptions(),r=fitNurbsOffsetEnvelope(options),expected=envelopeFitExpectation(options)
 expect(r.qualification!.partition.length).toBeGreaterThan(1)
 for(const key of ['wholeCurveComplete','tangentToleranceProven','trimMembershipProven','embeddingProven','topologyAuthority']){
  expect(validEnvelopeFit(expected,{...r,[key]:true})).toBe(false)
  const promoted=structuredClone(r);Object.assign(promoted.qualification!,{[key]:true});expect(validEnvelopeFit(expected,promoted)).toBe(false)
 }
 const missing=structuredClone(r);missing.qualification!.cells.pop();expect(validEnvelopeFit(expected,missing)).toBe(false)
 const orphan=structuredClone(r);orphan.qualification!.partition.pop();expect(validEnvelopeFit(expected,orphan)).toBe(false)
 const cycle=structuredClone(r);cycle.qualification!.partition[0].children![0]=0;expect(validEnvelopeFit(expected,cycle)).toBe(false)
 const boundary=structuredClone(r);boundary.qualification!.partition[1].domain[1][0]+=1e-4;expect(validEnvelopeFit(expected,boundary)).toBe(false)
 const error=structuredClone(r);error.qualification!.cells[0].errorUpperMm=options.toleranceMm*2;expect(validEnvelopeFit(expected,error)).toBe(false)
 const budget=structuredClone(r);budget.qualification!.envelopeQueries=0;expect(validEnvelopeFit(expected,budget)).toBe(false)
 expect(validEnvelopeFit(expected,{...r,candidateSurface:{controlPoints:{length:2,0:[]}}})).toBe(false)
})
it.each([false,true])('qualifies a finite patch on a curved contact, rotated=%s',rotated=>{
 const {options}=curvedTangentFixture(rotated),query={...options,maxCells:511,toleranceMm:1e-3},before=structuredClone(query),r=fitNurbsOffsetEnvelope(query)
 expect(r.qualification!.finiteNurbsPatchProven).toBe(true);expect(validEnvelopeFit(envelopeFitExpectation(query),r)).toBe(true)
 expect(r.candidateSurface!.degreeU).toBe(1);expect(r.candidateSurface!.degreeV).toBe(2)
 expect(query).toEqual(before)
},30_000)
it('keeps a failed section proposal unqualified and reports invalid inputs',()=>{
 const options=planeOptions(),query={...options,b:options.a,secondDomain:[[.30,.44],[.25,.35]] as [[number,number],[number,number]]},r=fitNurbsOffsetEnvelope(query)
 expect(r).toMatchObject({candidateSurface:null,qualification:null,reason:'proposal-contact-unresolved'})
 expect(validEnvelopeFit(envelopeFitExpectation(query),r)).toBe(true)
 expect(()=>fitNurbsOffsetEnvelope({...options,maxCells:0})).toThrow()
 expect(()=>fitNurbsOffsetEnvelope({...options,toleranceMm:0})).toThrow()
 expect(()=>fitNurbsOffsetEnvelope({...options,distances:[.2,.3]})).toThrow()
})
