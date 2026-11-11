import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {expect,it,vi} from 'vitest'
const native=vi.hoisted(()=>vi.fn())
vi.mock('../src/kernel/geometry/nurbs',()=>({callNurbsRust:native}))
import {restoreFrozenGraphBody,frozenGraphExpectation,validFrozenGraphBody} from '../src/kernel/frozenGraphBody'
const artifact=JSON.parse(gunzipSync(readFileSync('tests/fixtures/frozen-graph-body.json.gz')).toString())
function receipt():any {const g=artifact.geometry,n=g.faces.length;return {representation:'native-exact-graph-body',artifact:structuredClone(artifact),report:{method:'native-frozen-graph-body-restore/1',nativeVolumeCertified:true,globalEmbeddingCertified:true,faces:n,edges:g.edges.length,vertices:g.vertices.length,bodies:1,openEdges:0,pairProducts:n*(n-1)/2,allPairsReproved:true,internalG2Joins:1,curvatureLimitPerMm:10,absoluteCurvatureUpper:9,signedVolumeMm3:g.signedVolume,coefficientWork:100,maxWork:10000000,maxPairs:10000,maxBytes:4194304,radiusLawCertified:false,thicknessCertified:false,applicationAdmitted:false}}}
it('returns an exact native body carrier without rounded B-rep or application claims',()=>{
 const r=receipt();native.mockReturnValue(r)
 expect(restoreFrozenGraphBody(artifact)).toBe(r)
 expect(native).toHaveBeenLastCalledWith('brep_frozen_graph_body_restore',{artifact,maxBytes:4194304,maxWork:10000000,maxPairs:10000})
})
it('binds all exact geometry and identity links to an immutable request expectation',()=>{
 const input=structuredClone(artifact),expected=frozenGraphExpectation(input),r=receipt()
 input.geometry.vertices[0][0][0]=['999','1']
 expect(validFrozenGraphBody(r,expected)).toBe(true)
 const changes=[(r:any)=>r.artifact.geometry.identityLinks.edges[0].sourceId='different-id',(r:any)=>r.artifact.geometry.edges[0].range[0]=['1','7'],(r:any)=>r.report.applicationAdmitted=true,(r:any)=>r.report.radiusLawCertified=true,(r:any)=>r.report.pairProducts=0,(r:any)=>r.report.maxPairs=10001,(r:any)=>r.report.absoluteCurvatureUpper=11,(r:any)=>r.report.signedVolumeMm3=['0','1']]
 for(const change of changes){const v=receipt();change(v);expect(validFrozenGraphBody(v,expected)).toBe(false)}
})
it('does not expose a reply after cancellation',()=>{
 native.mockClear();expect(()=>restoreFrozenGraphBody(artifact,{},()=>{throw Error('cancelled')})).toThrow('cancelled');expect(native).not.toHaveBeenCalled()
 native.mockReturnValue(receipt());let calls=0;expect(()=>restoreFrozenGraphBody(artifact,{},()=>{if(++calls===2)throw Error('cancelled')})).toThrow('cancelled')
})
it('routes the exact body through worker result validation',async()=>{
 const {mainSolidExpectation,mainSolidResult}=await import('../src/kernel/mainSolidProtocol')
 const expected=mainSolidExpectation({kind:'frozenGraphBodyRestore',artifact,options:{}})
 expect(mainSolidResult(expected,receipt())).toBe(true)
 const r=receipt();r.artifact.geometry.identityLinks.faces[0][0]='changed';expect(mainSolidResult(expected,r)).toBe(false)
})
it('binds versioned natural-cut artifacts to the requested format through worker validation',async()=>{
 const v2=structuredClone(artifact);v2.format='native-convex-graph-volume/2';v2.recipe.naturalCuts=[{root:10,at:['1','3']}]
 const r=receipt();r.artifact=v2;native.mockReturnValue(r)
 expect(restoreFrozenGraphBody(v2)).toBe(r)
 const {mainSolidExpectation,mainSolidResult}=await import('../src/kernel/mainSolidProtocol')
 const expected=mainSolidExpectation({kind:'frozenGraphBodyRestore',artifact:v2,options:{}})
 expect(mainSolidResult(expected,r)).toBe(true)
 const downgrade=structuredClone(r);downgrade.artifact.format='native-convex-graph-volume/1'
 expect(mainSolidResult(expected,downgrade)).toBe(false)
 expect(()=>frozenGraphExpectation({...v2,format:'native-convex-graph-volume/3'})).toThrow()
})
