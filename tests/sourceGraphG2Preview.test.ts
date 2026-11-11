import {expect,it,vi} from 'vitest'
const native=vi.hoisted(()=>vi.fn())
vi.mock('../src/kernel/geometry/nurbs',()=>({callNurbsRust:native}))
import {createSourceGraphG2Preview} from '../src/kernel/sourceGraphG2Preview'
const surface={degreeU:1,degreeV:6,knotsU:[0,0,1,1],knotsV:[...Array(7).fill(0),...Array(7).fill(1)],controlPoints:[0,1].map(x=>Array.from({length:7},(_,y)=>[x,y/6,0])),weights:[Array(7).fill(1),Array(7).fill(1)]}
const width={degree:1,knots:[0,0,1,1],controlPoints:[[.1,0,0],[.1,0,0]],weights:[1,1]}
const options={radiusMm:1,radiusToleranceMm:.001}
function receipt():any{return {patch:structuredClone(surface),report:{method:'native-source-graph-convex-G2-preview/1',representation:'rounded-display',sourceContactJets:[true,true],previewStorageErrorMm:1e-12,curvatureLimitPerMm:10,curvatureDecision:'within',absoluteCurvatureUpper:9,radiusDomain:[[0,1],[0,1]],radiusDecision:'exceeds',radiusMm:1,radiusToleranceMm:.001,radiusWitnessUv:[[.25,.25],[.25,.25]],radiusWitnessMm:[.8,.9],coefficientWork:100,maxWork:1000000,solidGeometryCertified:false,interiorEmbeddingCertified:false,applicationAdmitted:false}}}
it('preserves full-domain radius refusal while returning a display-only candidate',()=>{
 const r=receipt();native.mockReturnValue(r)
 expect(createSourceGraphG2Preview([surface,surface],width,options)).toBe(r)
 expect(native).toHaveBeenLastCalledWith('brep_source_graph_g2_preview',expect.objectContaining({maxWork:1000000,maxCells:1000}))
})
it('refuses forged admission, missing geometry, stale radii and partial domains',()=>{
 for(const mutate of [(r:any)=>r.report.applicationAdmitted=true,(r:any)=>r.report.solidGeometryCertified=true,(r:any)=>r.report.coefficientWork=1000001,(r:any)=>r.report.radiusDomain=[[0,1],[.25,.75]],(r:any)=>r.report.radiusMm=2,(r:any)=>r.report.radiusWitnessUv=null,(r:any)=>r.patch.weights=[],(r:any)=>r.report.absoluteCurvatureUpper=11]){
  const r=receipt();mutate(r);native.mockReturnValue(r);expect(()=>createSourceGraphG2Preview([surface,surface],width,options)).toThrow()
 }
})
it('checks cancellation before native work and before exposing its reply',()=>{
 native.mockClear();expect(()=>createSourceGraphG2Preview([surface,surface],width,options,()=>{throw Error('cancelled')})).toThrow('cancelled');expect(native).not.toHaveBeenCalled()
 native.mockReturnValue(receipt());let visits=0
 expect(()=>createSourceGraphG2Preview([surface,surface],width,options,()=>{if(++visits===2)throw Error('cancelled')})).toThrow('cancelled')
})

it('binds worker reply validation to the requested radius and work budget',async()=>{
 const {mainSolidExpectation,mainSolidResult}=await import('../src/kernel/mainSolidProtocol')
 const job={kind:'sourceGraphG2Preview' as const,sources:[surface,surface] as [typeof surface,typeof surface],width,options}
 const expected=mainSolidExpectation(job)
 expect(mainSolidResult(expected,receipt())).toBe(true)
 const wrong=receipt();wrong.report.radiusMm=2
 expect(mainSolidResult(expected,wrong)).toBe(false)
 const promoted=receipt();promoted.report.applicationAdmitted=true
 expect(mainSolidResult(expected,promoted)).toBe(false)
})
