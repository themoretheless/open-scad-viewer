import {it,expect} from 'vitest'
import {createBrepBox,createBrepCylinder,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {certifyNurbsMinkowski,verifyMinkowskiProof,certifyMinkowski3d,type NurbsMinkowskiProof} from '../src/services/geometry/minkowski3d'
import {certifyBrepSpanIntersections,verifyBrepSpanIntersections} from '../src/services/geometry/brepProfile'
import {applySolidMinkowski} from '../src/services/solidMinkowski'
import {mainSolidResult,mainSolidExpectation} from '../src/services/mainSolidProtocol'
import {normalizePolygonMesh} from '../src/services/geometry/polygon'
import {box3d} from './fixtures/minkowski3d-cases'
import {rationalRoof,constantHalf,squareParameter,retracedParameter,planarCurve,constantMultispan} from './fixtures/certified-followup-cases'
it('binds a rational NURBS source to continuous bounds, filled enclosures and the rounded embedding',()=>{
 const a={kind:'brep' as const,model:rationalRoof(createBrepBox([0,0,0],[2,2,2]))},b={kind:'mesh' as const,mesh:box3d([-1,-1,-1],[1,1,1])}
 const r=certifyNurbsMinkowski(a,b,1,'f32'),proof:NurbsMinkowskiProof={version:2,a,b,certificate:r.certificate,meshPrecision:'f32'}
 expect(r.curvedSourceErrorCertified).toBe(true);expect(r.nominalIsExactCurvedSum).toBe(false);expect(r.sourceSurfaceDeviationMm[0]).toBeGreaterThan(0);expect(r.sourceSurfaceDeviationMm[0]).toBeLessThanOrEqual(1)
 expect(r.maximumSetDeviationMm).not.toBeNull();expect(r.certificate.inner.vertices.length).toBeGreaterThan(0);expect(r.certificate.outer.vertices).not.toEqual(r.certificate.nominal.vertices)
 expect(r.approximationTopology).toMatchObject({status:'proved',kind:'radial-minkowski-homeomorphism',includesMeshRounding:true});expect(r.roundingTopology).toMatchObject({status:'proved',precision:'f32',wholeLinearHomotopy:true});expect(verifyMinkowskiProof(JSON.parse(JSON.stringify(proof))).mesh).toEqual(r.mesh)
 for(const mutation of [(p:NurbsMinkowskiProof)=>{p.a.kind==='brep'&&(p.a.model.faces[1].surface.weights[1][1]=1)},(p:NurbsMinkowskiProof)=>{p.certificate.sourceHomotopyBounds[0]=['0','1']},(p:NurbsMinkowskiProof)=>{p.certificate.inner.vertices=[]},(p:NurbsMinkowskiProof)=>{p.certificate.filledHausdorffBound=['0','1']}]){const bad=structuredClone(proof);mutation(bad);expect(()=>verifyMinkowskiProof(bad)).toThrow()}
},120000)
it('uses source B-rep definitions in the document transaction and keeps a recheckable proof',()=>{
 const bodies=[createBrepBox([0,0,0],[2,3,4]),createBrepBox([-1,-1,-1],[1,1,1])].map((brep,i)=>({id:i?'b':'a',name:i?'B':'A',brep,mesh:tessellateNurbsBrep(brep,1)})),document={version:1 as const,sketches:[],bodies},before=structuredClone(document),options={ids:['a','b'] as [string,string],createdId:'sum',ru:true,maximumSourceDeviationMm:.1}
 const r=applySolidMinkowski(document,options);expect(r.proof.version).toBe(2);expect(r.curvedSourceErrorCertified).toBe(true);expect(r.maximumSetDeviationMm).toBe(0);expect(r.volumeMm3).toBe(120);expect(document).toEqual(before)
 expect(mainSolidResult(mainSolidExpectation({kind:'minkowski3d',document,options}),r)).toBe(true);expect(verifyMinkowskiProof(r.proof).mesh).toEqual(r.document.bodies[2].mesh)
 const forged=structuredClone(r);forged.approximationTopology={status:'proved',kind:'radial-minkowski-homeomorphism',sourceCenters:[[['999','1']],[['0','1']]],filledSet:'closed-three-ball',boundary:'two-sphere',includesMeshRounding:true};expect(mainSolidResult(mainSolidExpectation({kind:'minkowski3d',document,options}),forged)).toBe(false)
})
it('certifies a genuinely trimmed cylinder and preserves its source definitions',()=>{
 const brep=createBrepCylinder(1,2),document={version:1 as const,sketches:[],bodies:[{id:'a',name:'Cylinder',brep,mesh:tessellateNurbsBrep(brep,1)},{id:'b',name:'Kernel',mesh:normalizePolygonMesh(box3d([0,0,0],[1,1,1]))}]},before=structuredClone(document)
 const r=applySolidMinkowski(document,{ids:['a','b'],createdId:'sum',ru:true,maximumSourceDeviationMm:2});expect(r.document.bodies).toHaveLength(3);expect(document).toEqual(before);expect(r.approximationTopology?.status).toBe('proved');expect([2,3,4]).toContain(r.proof.certificate.version)
 const bad=structuredClone(r.proof) as NurbsMinkowskiProof;bad.certificate.sourceTopology![0]!.center[0]=['999','1'];expect(()=>verifyMinkowskiProof(bad)).toThrow()
},120000)
it('refuses a thin feature that collapses in f32 and proves nonzero safe rounding',()=>{
 const a=box3d([1,0,0],[1+2**-30,1,1]);expect(certifyMinkowski3d(a,a,'f64').volumeMm3).toBeGreaterThan(0);expect(()=>certifyMinkowski3d(a,a,'f32')).toThrow(/collapse|reverse/)
 const b={positions:[0,0,0,1,0,0,0,1,0,0,0,1],indices:[0,2,1,0,1,3,0,3,2,1,2,3]},r=certifyMinkowski3d(b,box3d([0,0,0],[1,1,1]),'f32')
 expect(r.roundingTopology).toMatchObject({status:'proved',precision:'f32',identity:false,wholeLinearHomotopy:true});expect(r.roundingTopology.pairs).toBeGreaterThan(0)
})
it('represents infinite parameter fibers in both directions and rejects a missing locus',()=>{
 for(const [a,b]of [[constantHalf(),squareParameter()],[squareParameter(),constantHalf()]]){const c=certifyBrepSpanIntersections(a,b),r=verifyBrepSpanIntersections(a,b,c);expect(c.version).toBe(3);expect(r.finiteEventsExhaustive).toBe(false);expect(r.algebraicLoci.length).toBe(1);expect(r.algebraicLoci[0].wholeSquare).toBe(false)
  const bad=structuredClone(c);bad.pairs[0].relation=[];expect(()=>verifyBrepSpanIntersections(a,b,bad)).toThrow()}
})
it('retains a constant self square, complete stationary span and nonmonotone self locus',()=>{
 const a=constantHalf(),c=certifyBrepSpanIntersections(a,a,true),r=verifyBrepSpanIntersections(a,a,c);expect(r.algebraicLoci[0]).toMatchObject({wholeSquare:true,diagonalExcluded:true});expect(r.stationarySpans).toEqual([[0,1]])
 const b=retracedParameter(),s=verifyBrepSpanIntersections(b,b,certifyBrepSpanIntersections(b,b,true));expect(s.algebraicLoci.length).toBe(1);expect(s.singularParameters[0][0]).toBeLessThanOrEqual(.5);expect(s.singularParameters[0][1]).toBeGreaterThanOrEqual(.5)
 const multi=constantMultispan(),mr=verifyBrepSpanIntersections(multi,multi,certifyBrepSpanIntersections(multi,multi,true));expect(mr.algebraicLoci).toHaveLength(3);expect(mr.algebraicLoci.every(l=>l.diagonalExcluded)).toBe(true);expect(mr.stationarySpans).toEqual([[0,.5],[.5,1]])
 const other=planarCurve([[2,0],[2,0]]),e=verifyBrepSpanIntersections(a,other,certifyBrepSpanIntersections(a,other));expect(e.events).toEqual([]);expect(e.algebraicLoci).toEqual([])
})

it('binds branch connectivity and deleted diagonal nodes to the exact relation',()=>{
 const a=retracedParameter(),c=certifyBrepSpanIntersections(a,a,true),r=verifyBrepSpanIntersections(a,a,c)
 const g=r.branchDecompositions[0].decomposition;expect(g.dimension).toBe(1);expect(g.cuts).toHaveLength(3);expect(g.slabs.map(s=>s.roots.length)).toEqual([1,1]);expect(g.cuts[1].excludedNodes).toEqual([true]);expect(g.cuts[1].left).toEqual([0]);expect(g.cuts[1].right).toEqual([0])
 const bad=structuredClone(c);bad.pairs[0].branches!.cuts[1].right=[];expect(()=>verifyBrepSpanIntersections(a,a,bad)).toThrow()
 const legacy=structuredClone(c);legacy.version=2;legacy.pairs.forEach(p=>delete p.branches);expect(verifyBrepSpanIntersections(a,a,legacy).branchDecompositions).toEqual([])
})
