import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {beforeAll,expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import type {NurbsSurface} from '../src/services/nurbsSurface'
import {certifyNurbsOffsetSourceBoundary,certifyTrimmedNurbsOffsetContactBand,certifyNurbsOffsetContactBand,evaluateNurbsSurfaceOffset,boundNurbsSurfaceOffset,boundNurbsSurfaceOffsetJacobian,certifyNurbsOffsetContactSection,findNurbsOffsetCandidateBoxes} from '../src/services/nurbsSurfaceOffset'
beforeAll(async()=>{await warmGeometryKernel()})
const plane=():NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false})
function contains(bounds:number[][],point:number[]){point.forEach((x,k)=>{expect(bounds[k][0]).toBeLessThanOrEqual(x);expect(bounds[k][1]).toBeGreaterThanOrEqual(x)})}
it('evaluates and encloses source offsets through actual WASM without changing the source',()=>{
 const s=plane(),before=structuredClone(s)
 const p=evaluateNurbsSurfaceOffset(s,[.37,.62],.2)
 expect(p).toMatchObject({point:[.37,.62,.2],du:[1,0,0],dv:[0,1,0],certified:false,topologyAuthority:false})
 const r=boundNurbsSurfaceOffset(s,[[0,1],[0,1]],.2,1);contains(r.image!,p.point)
 const j=boundNurbsSurfaceOffsetJacobian(s,[[0,1],[0,1]],.2,1)
 contains(j.derivatives![0],p.du);contains(j.derivatives![1],p.dv)
 expect(j.continuityCertified).toBe(false);expect(j.offsetRegularityCertified).toBe(false)
 expect(s).toEqual(before)
})
it('returns a certified section center while retaining the missing whole-curve and trim gates',()=>{
 const a=plane(),b=plane();for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 const options={a,b,distances:[.2,.2] as [number,number],fixedAxis:0 as const,fixed:.37,firstOther:[.25,.35] as [number,number],secondDomain:[[.32,.42],[.15,.25]] as [[number,number],[number,number]],maxSpans:2}
 const before=structuredClone(options),r=certifyNurbsOffsetContactSection(options)
 expect(r.status).toBe('unique-contact');expect(r.rootExistenceProven).toBe(true)
 contains(r.witness!.centerIntervalMm,[.37,.3,.2]);expect(r.witness!.contractionUpper).toBeLessThan(.5)
 expect(r.wholeCurveComplete).toBe(false);expect(r.trimMembershipProven).toBe(false);expect(r.topologyAuthority).toBe(false)
 expect(options).toEqual(before)
 expect(certifyNurbsOffsetContactSection({...options,b:a,secondDomain:[[.32,.42],[.25,.35]]}).status).toBe('unresolved')
})
it('keeps unvisited candidate boxes and refuses invalid second surfaces',()=>{
 const a=plane(),options={a,b:plane(),domains:[[[0,1],[0,1]],[[0,1],[0,1]]] as [[[number,number],[number,number]],[[number,number],[number,number]]],distances:[.2,.2] as [number,number],parameterTolerance:.01,maxBoxes:1,maxSpans:1}
 const r=findNurbsOffsetCandidateBoxes(options)
 expect(r.reason).toBe('work-limit');expect(r.visitedBoxes).toBe(1);expect(r.pendingBoxes).toHaveLength(2)
 expect(r.rootExistenceProven).toBe(false);expect(r.topologyAuthority).toBe(false)
 const invalid=plane();invalid.weights[0][0]=0
 expect(()=>findNurbsOffsetCandidateBoxes({...options,b:invalid})).toThrow()
})

it('proves a continuous branch over the driving interval and refuses a tube missing its endpoints',()=>{
 const a=plane(),b=plane();for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 const options={a,b,distances:[.2,.2] as [number,number],fixedAxis:0 as const,fixedInterval:[.35,.39] as [number,number],firstOther:[.25,.35] as [number,number],secondDomain:[[.30,.44],[.15,.25]] as [[number,number],[number,number]],maxSpans:2}
 const before=structuredClone(options),r=certifyNurbsOffsetContactBand(options)
 expect(r).toMatchObject({status:'continuous-branch',rootForEveryParameterProven:true,uniqueWithinTube:true,continuousBranchProven:true,wholeCurveComplete:false,trimMembershipProven:false,topologyAuthority:false})
 for(const t of [.35,.351,.37,.389,.39])contains(r.witness!.centerIntervalMm,[t,.3,.2])
 expect(options).toEqual(before)
 const expectation=mainSolidExpectation({kind:'offsetContactBand',options})
 expect(mainSolidResult(expectation,r)).toBe(true)
 expect(mainSolidResult(expectation,{...r,wholeCurveComplete:true})).toBe(false)
 expect(mainSolidResult(expectation,{...r,continuousBranchProven:false})).toBe(false)
 expect(mainSolidResult(expectation,{...r,witness:{...r.witness!,firstUV:[[.36,.38],r.witness!.firstUV[1]]}})).toBe(false)
 expect(mainSolidResult(expectation,{...r,witness:{...r.witness!,contractionUpper:.5}})).toBe(false)
 const narrow=certifyNurbsOffsetContactBand({...options,secondDomain:[[.36,.38],[.15,.25]]})
 expect(narrow).toMatchObject({status:'unresolved',rootForEveryParameterProven:false,continuousBranchProven:false,witness:null})
})

it('admits the entire offset band on authored trims and rejects contact inside a hole',()=>{
 const a=plane(),b=plane();for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 const rectangle=(lo:[number,number],hi:[number,number],reverse=false)=>{
  const p=[lo,[hi[0],lo[1]],hi,[lo[0],hi[1]]];if(reverse)p.reverse()
  return p.map((point,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[point,p[(i+1)%4]],weights:[1,1],periodic:false}))
 }
 const outer=rectangle([0,0],[1,1]),options={a,b,distances:[.2,.2] as [number,number],fixedAxis:0 as const,fixedInterval:[.35,.39] as [number,number],firstOther:[.25,.35] as [number,number],secondDomain:[[.30,.44],[.15,.25]] as [[number,number],[number,number]],maxSpans:2,firstLoops:[outer],secondLoops:[outer],toleranceUv:1e-7,maxPairs:10000,maxCells:10000,maxDomainCells:100000}
 const before=structuredClone(options),r=certifyTrimmedNurbsOffsetContactBand(options)
 expect(r).toMatchObject({trimMembershipProven:true,continuousBranchProven:true,worldCoedgeIdentityProven:false,topologyAuthority:false})
 expect(r.classifications.map(c=>c.location)).toEqual(['inside','inside'])
 const expected=mainSolidExpectation({kind:'trimmedOffsetContactBand',options})
 expect(mainSolidResult(expected,r)).toBe(true)
 expect(mainSolidResult(expected,{...r,worldCoedgeIdentityProven:true})).toBe(false)
 expect(mainSolidResult(expected,{...r,regions:[r.regions[0]]})).toBe(false)
 expect(mainSolidResult(expected,{...r,domainCells:r.domainCells+1})).toBe(false)
 expect(mainSolidResult(expected,{...r,classifications:[r.classifications[0],{...r.classifications[1],location:'outside'}]})).toBe(false)
 expect(options).toEqual(before)
 const hole=certifyTrimmedNurbsOffsetContactBand({...options,firstLoops:[outer,rectangle([.34,.29],[.40,.31],true)]})
 expect(hole).toMatchObject({trimMembershipProven:false,reason:'contact-outside-trim'})
 expect(mainSolidResult(expected,hole)).toBe(true)
 expect(mainSolidResult(expected,{...hole,trimMembershipProven:true})).toBe(false)
 const crossing=certifyTrimmedNurbsOffsetContactBand({...options,firstLoops:[rectangle([.36,.1],[.9,.9])]})
 expect(crossing).toMatchObject({trimMembershipProven:false,reason:'contact-trim-unresolved'})
 const cap=certifyTrimmedNurbsOffsetContactBand({...options,maxPairs:1,maxCells:1,maxDomainCells:1})
 expect(cap.trimMembershipProven).toBe(false);expect(cap.pairs).toBeLessThanOrEqual(1);expect(cap.cells).toBeLessThanOrEqual(1);expect(cap.domainCells).toBeLessThanOrEqual(1)
})

it('links every source spatial coedge while keeping exact identity and tolerance agreement separate',()=>{
 const a=plane(),b=plane();for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 const p=[[0,0],[1,0],[1,1],[0,1]],outer=p.map((point,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[point,p[(i+1)%4]],weights:[1,1],periodic:false}))
 const world=(side:number)=>outer.map(c=>({world:{...c,controlPoints:c.controlPoints.map(uv=>side===0?[uv[0],uv[1],0]:[uv[0],.5,uv[1]])},reversed:false}))
 const options={a,b,distances:[.2,.2] as [number,number],fixedAxis:0 as const,fixedInterval:[.35,.39] as [number,number],firstOther:[.25,.35] as [number,number],secondDomain:[[.30,.44],[.15,.25]] as [[number,number],[number,number]],maxSpans:2,firstLoops:[outer],secondLoops:[outer],firstCoedges:[world(0)],secondCoedges:[world(1)],toleranceUv:1e-7,maxPairs:10000,maxCells:10000,maxDomainCells:100000,toleranceMm:1e-5,maxExactWork:1000000,maxAgreementCells:10000}
 const before=structuredClone(options),r=certifyNurbsOffsetSourceBoundary(options)
 expect(r).toMatchObject({worldCoedgeIdentityProven:true,worldBoundaryWithinToleranceProven:true,checkedCoedges:8,totalCoedges:8,topologyAuthority:false,wholeCurveComplete:false})
 expect(options).toEqual(before)
 const expected=mainSolidExpectation({kind:'offsetSourceBoundary',options})
 expect(mainSolidResult(expected,r)).toBe(true)
 expect(mainSolidResult(expected,{...r,topologyAuthority:true})).toBe(false)
 expect(mainSolidResult(expected,{...r,audits:r.audits.slice(0,-1)})).toBe(false)
 expect(mainSolidResult(expected,{...r,exactWork:r.exactWork+1})).toBe(false)
 expect(mainSolidResult(expected,{...r,audits:[{...r.audits[0],side:1},...r.audits.slice(1)]})).toBe(false)
 const changed=structuredClone(options);changed.secondCoedges[0][0].world.controlPoints[0][1]=.501
 expect(certifyNurbsOffsetSourceBoundary(changed)).toMatchObject({worldCoedgeIdentityProven:false,worldBoundaryWithinToleranceProven:false,reason:'spatial-boundary-mismatch'})
 const limited=certifyNurbsOffsetSourceBoundary({...options,maxExactWork:0,maxAgreementCells:1})
 expect(limited).toMatchObject({worldCoedgeIdentityProven:false,worldBoundaryWithinToleranceProven:false,reason:'boundary-work-limit'})
 expect(limited.checkedCoedges).toBeLessThan(limited.totalCoedges)
 expect(mainSolidResult(expected,limited)).toBe(false)
 const limitedExpected=mainSolidExpectation({kind:'offsetSourceBoundary',options:{...options,maxExactWork:0,maxAgreementCells:1}})
 expect(mainSolidResult(limitedExpected,limited)).toBe(true)
 expect(mainSolidResult(limitedExpected,{...limited,worldBoundaryWithinToleranceProven:true})).toBe(false)
 expect(certifyNurbsOffsetSourceBoundary({...options,maxExactWork:0})).toMatchObject({worldCoedgeIdentityProven:false,worldBoundaryWithinToleranceProven:true})
 expect(()=>certifyNurbsOffsetSourceBoundary({...options,secondCoedges:[]})).toThrow()
})
