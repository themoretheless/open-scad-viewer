import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {callGeometryRust} from '../src/services/geometry/kernel'
import type {NurbsBrep} from '../src/services/geometry/brep'
import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

// Authored inputs exported by the Rust fixture executable; no geometry or
// proof decisions are implemented in this transport test.
const fixtures: {spatial:boolean;weighted:boolean;request:Record<string,unknown>}[] =
 JSON.parse(readFileSync('tests/fixtures/quintic-polygon-stations-6.json','utf8'))
const source:Record<string,unknown>=JSON.parse(readFileSync('tests/fixtures/quintic-polygon-rmf-source.json','utf8'))
interface BodyReceipt {
 accepted:boolean
 model:NurbsBrep|null
 stationContinuity:string
 profileContinuity:string
 continuousBound:boolean
 retainedBodyGeometryCertified:boolean
 globalEmbeddingCertified:boolean
 station:unknown
 volume:unknown
}
const build=(request:Record<string,unknown>)=>callGeometryRust<BodyReceipt>(
 'brep_nurbs_smooth_polygon_station_body',request)
it.each(fixtures)('transfers a freshly certified G2 station body: spatial=$spatial weighted=$weighted',fixture=>{
 const body=build(fixture.request)
 expect(body).toMatchObject({accepted:true,stationContinuity:'G2',profileContinuity:'C0',
  continuousBound:false,globalEmbeddingCertified:false,retainedBodyGeometryCertified:true,
  station:{stationG2Certified:true},volume:{solidGeometryCertified:true,boundaryEmbeddingCertified:true,
   allFacesInjective:true,allPairsClassified:true}})
 const document={nodes:[{id:'body',op:'brep_progressive_sweep'},{id:'placed',op:'transform',input:'body'}]}
 const model=body.model!
 const artifact=createNativeGeometryArtifact('placed','brep',model,document)
 expect(inspectProgressiveSweepSolidAdmission(artifact,model)).toMatchObject({solidGeometryCertified:true})
},180000)
it('refuses exhausted construction work and correction without publishing a partial model',()=>{
 const request=fixtures[3]!.request
 for(const maxWork of [0,1]) {
  expect(build({...request,maxWork})).toMatchObject({accepted:false,model:null})
 }
 expect(build({...request,maxDisplacement:0})).toMatchObject({accepted:false,model:null})
})
it('composes the original RMF source bound with fresh whole-body and G2 proofs',()=>{
 const request=fixtures[3]!.request
 expect(build({...request,source})).toMatchObject({accepted:true,continuousBound:true,
  retainedBodyGeometryCertified:true,globalEmbeddingCertified:false,
  station:{stationG2Certified:true},volume:{solidGeometryCertified:true},
  continuousCertificate:{withinBudget:true,scope:'all-retained-closed-body-walls'}})
 for(const maxCells of [0,1]) {
  expect(build({...request,source:{...source,maxCells}})).toMatchObject({accepted:false,model:null,
   continuousBound:false,continuousCertificate:{withinBudget:false,errorUpper:null}})
 }
},180000)
it('recomputes admission from changed source geometry rather than a retained receipt',()=>{
 const body=build(fixtures[3]!.request)
 const changed=structuredClone(body.model!)
 changed.faces[0]!.surface.controlPoints[0]![0]![0]!+=.0001
 const document={nodes:[{id:'body',op:'brep_progressive_sweep'}]}
 const artifact=createNativeGeometryArtifact('body','brep',{...changed,volume:{solidGeometryCertified:true}},document)
 expect(()=>inspectProgressiveSweepSolidAdmission(artifact,changed)).toThrow(/exact boundary agreement/)
},180000)
