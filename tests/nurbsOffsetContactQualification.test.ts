import {beforeAll,expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {fitNurbsOffsetEnvelope} from '../src/services/nurbsOffsetEnvelopeFit'
import {contactQualificationExpectation,qualifyNurbsOffsetContacts,validContactQualification,type OffsetContactQualificationOptions} from '../src/services/nurbsOffsetContactQualification'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
import type {MainSolidResponse} from '../src/services/mainSolidProtocol'
import {planeEnvelopeFixture} from './fixtures/offset-envelope'
beforeAll(async()=>{await warmGeometryKernel()})
function options():OffsetContactQualificationOptions {
 const base=planeEnvelopeFixture(),candidate=fitNurbsOffsetEnvelope({...base,toleranceMm:1e-3,maxCells:511}).candidateSurface!
 const line=(a:number[],b:number[])=>({degree:1,knots:[.35,.35,.39,.39],controlPoints:[a,b],weights:[1,1],periodic:false})
 const points=[[0,0],[1,0],[1,1],[0,1]]
 const loops=[points.map((p,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[p,points[(i+1)%4]],weights:[1,1],periodic:false}))]
 return {...base,candidate,sourcePcurves:[line([.35,.3],[.39,.3]),line([.35,.2],[.39,.2])],firstLoops:loops,secondLoops:loops,
  toleranceMm:1e-3,toleranceUv:1e-6,maxFitCells:511,maxUvCells:511,maxAgreementCells:511,rootRefinements:3,
  maxPairs:1000,maxTrimCells:10000,maxDomainCells:10000,maxSineSquared:1e-10,maxTangentPositionCells:10000,maxNormalCells:1000,maxNormalSpans:10000}
}
it('delivers full contact and tangent-plane qualification through actual WASM and worker runtime',async()=>{
 const o=options(),before=structuredClone(o),job={kind:'offsetContactQualification' as const,options:o}
 const r=qualifyNurbsOffsetContacts(o)
 expect(r.qualification.contactCurvesAndTangentPlanesProven).toBe(true)
 expect(validContactQualification(contactQualificationExpectation(o),r)).toBe(true)
 expect(mainSolidResult(mainSolidExpectation(job),r)).toBe(true)
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 await handle({version:1,id:1,job})
 expect(messages[0]).toMatchObject({kind:'offsetContactQualification',ok:true,result:r})
 expect(o).toEqual(before)
})
it('keeps angular work stops and rejects stale snapshots, candidate changes and omitted parameter coverage',()=>{
 const o=options(),e=contactQualificationExpectation(o),r=qualifyNurbsOffsetContacts(o)
 const stopped={...o,maxNormalSpans:1},stop=qualifyNurbsOffsetContacts(stopped)
 expect(stop.qualification.contactCurvesAndTangentPlanesProven).toBe(false)
 expect(validContactQualification(contactQualificationExpectation(stopped),stop)).toBe(true)
 expect(validContactQualification(e,stop)).toBe(false)
 const changed=structuredClone(r);changed.candidateSurface.controlPoints[0][0][2]+=.01
 expect(validContactQualification(e,changed)).toBe(false)
 const missing=structuredClone(r)
 const tangents=missing.qualification.tangentPlanes as {cells:unknown[]}[]
 tangents[0].cells=[]
 expect(validContactQualification(e,missing)).toBe(false)
 const angular=structuredClone(r)
 const cells=(angular.qualification.tangentPlanes as {cells:{sineSquaredInterval:[number,number]}[]}[])[0].cells
 cells[0].sineSquaredInterval=[.2,.3]
 expect(validContactQualification(e,angular)).toBe(false)
 const requestChanged=structuredClone(r);requestChanged.request.maxSineSquared=.5
 expect(validContactQualification(e,requestChanged)).toBe(false)
})
it('recovers the real worker after a busy request and malformed geometry',async()=>{
 const o=options(),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 const job={kind:'offsetContactQualification' as const,options:o}
 const first=handle({version:1,id:1,job})
 await handle({version:1,id:2,job})
 expect(messages[0]).toMatchObject({id:2,ok:false,error:{code:'CAD_BUSY'}})
 await first
 expect(messages[1]).toMatchObject({id:1,ok:true})
 const malformed=structuredClone(o);malformed.sourcePcurves[1].weights[0]=-1
 await handle({version:1,id:3,job:{...job,options:malformed}})
 expect(messages[2]).toMatchObject({id:3,ok:false})
 await handle({version:1,id:4,job})
 expect(messages[3]).toMatchObject({id:4,ok:true})
 if(messages[3].ok)expect(mainSolidResult(mainSolidExpectation(job),messages[3].result)).toBe(true)
})
it('retains incomplete fits and candidate mismatches without contact or tangent authority',()=>{
 const base=options()
 const stopped={...base,maxFitCells:1}
 const limited=qualifyNurbsOffsetContacts(stopped)
 expect(limited.qualification.contactCurvesAndTangentPlanesProven).toBe(false)
 expect(limited.qualification.tangentPlanes).toEqual([])
 expect(validContactQualification(contactQualificationExpectation(stopped),limited)).toBe(true)
 const changed=structuredClone(base)
 for(const row of changed.candidate.controlPoints)for(const p of row)p[2]+=.01
 const mismatch=qualifyNurbsOffsetContacts(changed)
 expect(mismatch.qualification.contactCurvesAndTangentPlanesProven).toBe(false)
 expect(mismatch.qualification.tangentPlanes).toEqual([])
 expect(validContactQualification(contactQualificationExpectation(changed),mismatch)).toBe(true)
})
