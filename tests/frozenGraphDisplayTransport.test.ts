import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {it,expect,vi} from 'vitest'
const native=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/geometry/nurbs',()=>({callNurbsRust:native}))
import {displayFrozenGraphBody} from '../src/services/frozenGraphBody'
const artifact=JSON.parse(gunzipSync(readFileSync('tests/fixtures/frozen-graph-body.json.gz')).toString())
function result():any{const g=artifact.geometry,n=g.faces.length;return {representation:'native-exact-graph-body',artifact:structuredClone(artifact),report:{method:'native-frozen-graph-body-restore/1',nativeVolumeCertified:true,globalEmbeddingCertified:true,faces:n,edges:g.edges.length,vertices:g.vertices.length,bodies:1,openEdges:0,pairProducts:n*(n-1)/2,allPairsReproved:true,internalG2Joins:1,curvatureLimitPerMm:10,absoluteCurvatureUpper:9,signedVolumeMm3:g.signedVolume,coefficientWork:100,maxWork:10000000,maxPairs:10000,maxBytes:4194304,radiusLawCertified:false,thicknessCertified:false,applicationAdmitted:false},display:{method:'native-exact-graph-edges/1',segments:8,edges:g.edges.map((e:any,edge:number)=>({edge,root:e.root,points:Array.from({length:9},()=>[0,0,0])})),sampleStorageErrorUpperMm:0,polylineDeviationCertified:false}}}
it('exposes stable edge slots without promoting display to a geometry certificate',()=>{const r=result();native.mockReturnValue(r);expect(displayFrozenGraphBody(artifact)).toBe(r);expect(native).toHaveBeenLastCalledWith('brep_frozen_graph_body_display',expect.objectContaining({artifact,segments:8}))})
it('refuses incomplete coverage, duplicated edges, wrong roots and invalid sample data',()=>{
 for(const mutate of [(r:any)=>r.display.edges.pop(),(r:any)=>r.display.edges[1].edge=0,(r:any)=>r.display.edges[0].root=-1,(r:any)=>r.display.edges[0].points[0][0]=NaN,(r:any)=>r.display.edges[0].points.pop(),(r:any)=>r.display.polylineDeviationCertified=true,(r:any)=>r.display.sampleStorageErrorUpperMm=-1]){
  const r=result();mutate(r);native.mockReturnValue(r);expect(()=>displayFrozenGraphBody(artifact)).toThrow()
 }
})
it('binds the sample count in the shared worker protocol',async()=>{
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const expected=mainSolidExpectation({kind:'frozenGraphBodyDisplay',artifact,segments:8,options:{}})
 expect(mainSolidResult(expected,result())).toBe(true)
 const r=result();r.display.segments=16;expect(mainSolidResult(expected,r)).toBe(false)
})
