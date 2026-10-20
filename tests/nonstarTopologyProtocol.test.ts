import {beforeAll,it,expect} from 'vitest'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {applySolidMinkowski,type SolidMinkowskiResult} from '../src/services/solidMinkowski'
import type {NurbsMinkowskiProof} from '../src/services/geometry/minkowski3d'
import {mainSolidResult,mainSolidExpectation} from '../src/services/mainSolidProtocol'
const bodies=[createBrepBox([0,0,0],[2,2,2]),createBrepBox([0,0,0],[1,1,1])].map((brep,i)=>({id:i?'b':'a',name:'body',brep,mesh:tessellateNurbsBrep(brep,1)}))
const document={version:1 as const,sketches:[],bodies},options={ids:['a','b'] as [string,string],createdId:'sum',ru:true},expected=mainSolidExpectation({kind:'minkowski3d',document,options})
let base:SolidMinkowskiResult
beforeAll(()=>{base=applySolidMinkowski(document,options)})
// Transport fixtures deliberately exercise schema/claim binding. Geometry is
// separately checked against original-source fixtures in Rust and raw WASM.
function sourceOnly(){const r=structuredClone(base),p=r.proof as NurbsMinkowskiProof,c=p.certificate
 c.version=3;c.sourceTopology=[null,null];c.sourceHomotopyBounds=[['1','1'],['0','1']];c.sourceAtlasTopology=[{kind:'signed-prismatic-ambient-isotopy',axis:2,levels:[['0','1'],['2','1']],shells:[{shell:0,interval:[['0','1'],['2','1']],curves:[0,1],coefficients:[1,-1]}],curveParents:[null,0],nominalProfileVertices:8,preservesComplement:true},null];c.sumTopology=null;r.approximationTopology=null;r.sourceApproximationTopology=[true,true];return r}
it('accepts a source ambient certificate without promoting it to a sum proof',()=>{
 const r=sourceOnly();expect(mainSolidResult(expected,r)).toBe(true)
 r.approximationTopology={status:'proved',kind:'prismatic-minkowski-homeomorphism',axis:2,survivingHoles:1,includesMeshRounding:true};expect(mainSolidResult(expected,r)).toBe(false)
})
it('binds the separate sum claim to the checked erosion receipt',()=>{
 const r=sourceOnly(),p=r.proof as NurbsMinkowskiProof
 p.certificate.sumTopology={kind:'convex-hole-prismatic-minkowski-homeomorphism',axis:2,interval:[['0','1'],['3','1']],holedOperand:0,holes:[{hole:1,open:true,depth:0,innerCore:[],outerCore:[]}],survivingHoles:1}
 r.approximationTopology={status:'proved',kind:'prismatic-minkowski-homeomorphism',axis:2,survivingHoles:1,includesMeshRounding:true};expect(mainSolidResult(expected,r)).toBe(true)
 r.approximationTopology.survivingHoles=0;expect(mainSolidResult(expected,r)).toBe(false)
})
it('rejects missing atlas reports, malformed forests and invented exact-source identity',()=>{
 for(const change of [(r:SolidMinkowskiResult)=>{(r.proof as NurbsMinkowskiProof).certificate.sourceAtlasTopology![0]=null},(r:SolidMinkowskiResult)=>{const a=(r.proof as NurbsMinkowskiProof).certificate.sourceAtlasTopology![0]!;if(a.kind==='signed-prismatic-ambient-isotopy')a.curveParents[1]=1},(r:SolidMinkowskiResult)=>{r.approximationTopology={status:'proved',kind:'exact-source-minkowski-identity',includesMeshRounding:true}}]){const r=sourceOnly();change(r);expect(mainSolidResult(expected,r)).toBe(false)}
})
it('transports a polynomial inverse atlas without a radial or sum assertion',()=>{
 const r=sourceOnly();(r.proof as NurbsMinkowskiProof).certificate.sourceAtlasTopology![0]={kind:'polynomial-shear-ambient-isotopy',axis:2,polynomial:[[['0','1']],[['1','2']]],sourceChartCells:6,referenceTriangles:48,homotopySegments:2,checkedPairs:1128,preservesComplement:true};expect(mainSolidResult(expected,r)).toBe(true)
 r.sourceApproximationTopology[0]=false;expect(mainSolidResult(expected,r)).toBe(false)
})

it('admits rational source atlases only with version 4 and preserves the separate sum requirement',()=>{
 const r=sourceOnly(),c=(r.proof as NurbsMinkowskiProof).certificate
 c.version=4;c.sourceAtlasTopology![0]={kind:'rational-atlas-ambient-isotopy',chartTriangles:128,vertexStars:64,disjointPairs:4096,boundaryPairs:512,preservesComplement:true}
 expect(mainSolidResult(expected,r)).toBe(true)
 c.version=3;expect(mainSolidResult(expected,r)).toBe(false)
 c.version=4;r.approximationTopology={status:'proved',kind:'prismatic-minkowski-homeomorphism',axis:2,survivingHoles:1,includesMeshRounding:true};expect(mainSolidResult(expected,r)).toBe(false)
})
