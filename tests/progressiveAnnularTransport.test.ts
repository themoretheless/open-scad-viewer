import {beforeEach,expect,it,vi} from 'vitest'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import type {ProgressiveBrepBody} from '../src/services/geometry/brep'
const calls=vi.hoisted(()=>({rust:vi.fn(),audit:vi.fn()}))
vi.mock('../src/services/geometry/kernel',()=>({callGeometryRust:calls.rust}))
vi.mock('../src/services/nurbsSweepEmbedding',async original=>({
 ...await original<object>(),inspectSweepVolumeAndCharts:calls.audit,
}))
import {createProgressiveBrepProfileBody,readNativeSweepPlanarCapFaces,reconstructCertifiedProgressiveStations,transformNurbsBrep} from '../src/services/geometry/brep'
// Transport-only doubles. Rust unit/ABI and actual-model audits qualify geometry.
const curve:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[0,0,1]],weights:[1,1],periodic:false}
const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const model=()=>({faces:Array.from({length:24},()=>({})),vertices:[],edges:[],loops:[],shells:[],bodies:[],toleranceMm:1e-9})
const body=()=>({model:model(),capRepresentation:'radial-annular',capFaces:[16,17,18,19,20,21,22,23],planarCapFaces:[],
 retainedSmoothness:{walls:{method:'native-test-double'}}})
const options={normal:[1,0,0],initialSections:3,maxSections:3,maxDeviation:.01,annularCaps:{maxWork:50000,maxProducts:5000,maxPairs:500,maxCells:1000,smoothnessMaxWork:2000000}}
beforeEach(()=>{calls.rust.mockReset();calls.audit.mockReset()})
it('transports original annular budgets and actual native cap selection through placement',()=>{
 const response=body();calls.rust.mockReturnValue(response)
 const created=createProgressiveBrepProfileBody([[curve],[curve]],curve,law,{...law,values:[0,0]},options)
 expect(calls.rust.mock.calls[0]).toMatchObject(['brep_nurbs_progressive_profile_annular_body',{
  annular_cap_max_work:50000,annular_cap_max_products:5000,annular_cap_max_pairs:500,annular_cap_max_cells:1000,
  smoothness_max_work:2000000,max_deviation:.01,loops:[[curve],[curve]],path:curve,
 }])
 expect(created.profileSmoothness).toBe(response.retainedSmoothness.walls)
 expect(readNativeSweepPlanarCapFaces(created.model)).toEqual([])
 // Public report mutation cannot cause last-two-faces planar optimization.
 created.planarCapFaces!.push(23)
 const moved=model();calls.rust.mockReturnValueOnce(moved)
 expect(readNativeSweepPlanarCapFaces(transformNurbsBrep(created.model,[]))).toEqual([])
})
it('replays the private original annular source and audits all actual reconstructed faces',()=>{
 calls.rust.mockReturnValueOnce(body())
 const loops=structuredClone([[curve],[curve]])
 const created=createProgressiveBrepProfileBody(loops,curve,law,{...law,values:[0,0]},options)
 loops[0]![0]!.controlPoints[0]![0]=999
 created.capRepresentation=undefined;created.capFaces=[23];created.planarCapFaces=[23]
 const candidate=model()
 calls.rust.mockReturnValueOnce({planarCapFaces:[],stationReconstruction:{reason:'test-double',source:'original-progressive-profile-request',
  reconstruction:{model:candidate,wallDisplacementUpper:0,work:100},profileSmoothness:{},
  boundaryCertificate:{method:'retained-sweep-boundary-union',scope:'boundary-set-hausdorff',continuousBound:true,withinBudget:true,errorUpper:0,budget:.01,closed:false}}})
 calls.audit.mockReturnValue({retainedWallCharts:{allChartsCertified:true},solidGeometryCertified:false})
 const repair={quantum:.125,wallTolerance:1e-9,maxWork:1000000,smoothnessMaxWork:2000000}
 reconstructCertifiedProgressiveStations(created,repair)
 expect(calls.rust.mock.calls[1]![0]).toBe('brep_nurbs_progressive_profile_annular_body_reconstruct_stations')
 expect(calls.rust.mock.calls[1]![1].loops[0][0].controlPoints[0][0]).toBe(0)
 expect(calls.audit.mock.calls[0]![0]).toBe(candidate)
 expect(calls.audit.mock.calls[0]![1]).toEqual([])
 expect(()=>reconstructCertifiedProgressiveStations({...created} as ProgressiveBrepBody,repair)).toThrow('Missing owned progressive station source')
})
it('keeps legacy calls unchanged and refuses missing native annular receipts',()=>{
 calls.rust.mockReturnValueOnce({model:model()})
 const {annularCaps,...legacy}=options
 createProgressiveBrepProfileBody([[curve]],curve,law,law,legacy)
 expect(calls.rust.mock.calls[0]![0]).toBe('brep_nurbs_progressive_profile_body')
 expect(calls.rust.mock.calls[0]![1]).not.toHaveProperty('annular_cap_max_work')
 calls.rust.mockReturnValueOnce({model:model()})
 expect(()=>createProgressiveBrepProfileBody([[curve],[curve]],curve,law,law,options)).toThrow('Native annular body evidence is unavailable')
})
