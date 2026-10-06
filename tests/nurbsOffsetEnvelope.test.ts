import {beforeAll,expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {certifyNurbsOffsetEnvelope} from '../src/services/nurbsSurfaceOffset'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {planeEnvelopeFixture} from './fixtures/offset-envelope'
import {curvedTangentFixture} from './fixtures/offset-contact-tangent'
beforeAll(async()=>{await warmGeometryKernel()})
function contains(bounds:number[][],point:number[]){point.forEach((x,k)=>{expect(bounds[k][0]).toBeLessThanOrEqual(x);expect(bounds[k][1]).toBeGreaterThanOrEqual(x)})}
it('covers the complete center band and rational arc with actual WASM derivatives',()=>{
 const options=planeEnvelopeFixture(),before=structuredClone(options),r=certifyNurbsOffsetEnvelope(options)
 expect(r).toMatchObject({envelopeRegularityProven:true,radiusMm:.2,wholeCurveComplete:false,finiteNurbsPatchProven:false,trimMembershipProven:false,embeddingProven:false,topologyAuthority:false})
 expect(r.visitedCells).toBeLessThanOrEqual(options.maxCells)
 expect(mainSolidResult(mainSolidExpectation({kind:'offsetEnvelope',options}),r)).toBe(true)
 let end=0
 for(const c of r.cells){
  expect(c.arcParameter[0]).toBe(end);end=c.arcParameter[1];expect(c.areaSpeedIntervalMm2[0]).toBeGreaterThan(0)
  for(const t of options.fixedInterval)for(const s of [c.arcParameter[0],(c.arcParameter[0]+c.arcParameter[1])/2,c.arcParameter[1]]){
   const w=Math.SQRT1_2,den=(1-s)**2+2*w*s*(1-s)+s*s
   const ny=2*w*s*(1-s)+s*s,nz=(1-s)**2+2*w*s*(1-s),dny=2*w*(1-2*s)+2*s,dnz=-2*(1-s)+2*w*(1-2*s),dd=dnz+2*s
   contains(c.imageIntervalsMm,[t,.3+.2*ny/den,.2-.2*nz/den])
   contains(c.centerDerivativeIntervalsMm,[1,0,0])
   const ps=[0,.2*(dny*den-ny*dd)/(den*den),-.2*(dnz*den-nz*dd)/(den*den)]
   contains(c.arcDerivativeIntervalsMm,ps)
   const area=Math.hypot(ps[1],ps[2]);expect(c.areaSpeedIntervalMm2[0]).toBeLessThanOrEqual(area);expect(c.areaSpeedIntervalMm2[1]).toBeGreaterThanOrEqual(area)
  }
 }
 expect(end).toBe(1);expect(options).toEqual(before)
})
it('retains the whole unresolved arc on a work stop and refuses malformed radii',()=>{
 const options={...planeEnvelopeFixture(),maxCells:1},r=certifyNurbsOffsetEnvelope(options)
 expect(r).toMatchObject({visitedCells:1,envelopeRegularityProven:false,reason:'envelope-regularity-unproven'})
 expect(r.cells).toHaveLength(1);expect(r.cells[0].arcParameter).toEqual([0,1])
 expect(mainSolidResult(mainSolidExpectation({kind:'offsetEnvelope',options}),r)).toBe(true)
 for(const distances of [[0,0],[.2,.3]] as [number,number][]){expect(()=>certifyNurbsOffsetEnvelope({...options,distances})).toThrow()}
 expect(()=>certifyNurbsOffsetEnvelope({...options,maxCells:0})).toThrow()
 const unresolvedOptions={...options,b:options.a,secondDomain:[[.30,.44],[.25,.35]] as [[number,number],[number,number]]},unresolved=certifyNurbsOffsetEnvelope(unresolvedOptions)
 expect(unresolved).toMatchObject({envelopeRegularityProven:false,visitedCells:0,cells:[],reason:'center-regularity-unproven'})
 expect(mainSolidResult(mainSolidExpectation({kind:'offsetEnvelope',options:unresolvedOptions}),unresolved)).toBe(true)
})
it('rejects omitted arc cells, promoted topology, wrong radius and inconsistent budgets',()=>{
 const options=planeEnvelopeFixture(),r=certifyNurbsOffsetEnvelope(options),expected=mainSolidExpectation({kind:'offsetEnvelope',options})
 expect(r.cells.length).toBeGreaterThan(1)
 for(const key of ['wholeCurveComplete','finiteNurbsPatchProven','trimMembershipProven','embeddingProven','topologyAuthority'])expect(mainSolidResult(expected,{...r,[key]:true})).toBe(false)
 expect(mainSolidResult(expected,{...r,radiusMm:.3})).toBe(false)
 expect(mainSolidResult(expected,{...r,visitedCells:options.maxCells+1})).toBe(false)
 expect(mainSolidResult(expected,{...r,cells:r.cells.slice(1),visitedCells:2*(r.cells.length-1)-1})).toBe(false)
 const gap=structuredClone(r);gap.cells[1].arcParameter[0]+=1e-5;expect(mainSolidResult(expected,gap)).toBe(false)
 const zero=structuredClone(r);zero.cells[0].areaSpeedIntervalMm2[0]=0;expect(mainSolidResult(expected,zero)).toBe(false)
 const fake=structuredClone(r);fake.cells[0].arcDerivativeIntervalsMm=[[-1,1],[-1,1],[-1,1]];expect(mainSolidResult(expected,fake)).toBe(false)
 const parallel=structuredClone(r);parallel.cells[0].centerDerivativeIntervalsMm=[[1,1],[0,0],[0,0]];parallel.cells[0].arcDerivativeIntervalsMm=[[1,1],[0,0],[0,0]];expect(mainSolidResult(expected,parallel)).toBe(false)
})
it.each([false,true])('qualifies the full arc on a curved contact, rotated=%s',rotated=>{
 const {options}=curvedTangentFixture(rotated),query={...options,maxCells:511},before=structuredClone(query),r=certifyNurbsOffsetEnvelope(query)
 expect(r.envelopeRegularityProven).toBe(true)
 expect(r.cells.every(c=>c.regularityProven&&c.areaSpeedIntervalMm2[0]>0)).toBe(true)
 expect(mainSolidResult(mainSolidExpectation({kind:'offsetEnvelope',options:query}),r)).toBe(true)
 expect(query).toEqual(before)
})
