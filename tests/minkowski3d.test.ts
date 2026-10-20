import {it,expect} from 'vitest'
import {certifyMinkowski3d,verifyMinkowski3d} from '../src/services/geometry/minkowski3d'
import {normalizePolygonMesh,inspectPolygonMesh} from '../src/services/geometry/polygon'
import {applySolidMinkowski} from '../src/services/solidMinkowski'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
import type {MainSolidResponse} from '../src/services/mainSolidProtocol'
import {mainSolidResult,mainSolidExpectation} from '../src/services/mainSolidProtocol'
import {box3d,lPrism3d,cavity3d,join3d,tetra3d} from './fixtures/minkowski3d-cases'
it('certifies nonconvex material and a cavity without taking the convex hull',()=>{
 const b=box3d([-1,-1,-1],[1,1,1])
 for(const [a,volume] of [[lPrism3d(),108],[cavity3d(),1720]] as const){const r=certifyMinkowski3d(a,b);expect(r.status).toBe('verified');expect(r.volumeMm3).toBeCloseTo(volume,8);expect(r.certificate.sourceCells[0]).toBeGreaterThan(1)
  expect(verifyMinkowski3d({version:1,a,b,certificate:r.certificate}).volumeMm3).toBeCloseTo(volume,8)
  expect(inspectPolygonMesh(r.mesh)).toMatchObject({closed:true,boundaryEdges:0,nonManifoldEdges:0,orientationConflicts:0,degenerateTriangles:0})
 }
},120000)
it('checks disconnected components, a slanted hull and exact output rounding bounds',()=>{
 const a=join3d(box3d([0,0,0],[1,1,1]),box3d([5,0,0],[6,1,1])),b=box3d([0,0,0],[1,1,1])
 expect(certifyMinkowski3d(a,b).volumeMm3).toBeCloseTo(16,8)
 const slanted=structuredClone(b);for(let i=0;i<slanted.positions.length;i+=3)slanted.positions[i]+=slanted.positions[i+1]
 expect(certifyMinkowski3d(box3d([0,0,0],[2,2,2]),slanted).volumeMm3).toBeCloseTo(33,8)
 const r=certifyMinkowski3d(tetra3d(),b);expect(r.volumeMm3).toBeCloseTo(17/3,8);expect(r.maximumCoordinateRoundingMm).toBeGreaterThan(0);expect(r.displayCoordinatesExact).toBe(false)
})
it('rejects altered source, boundary, census, schema and invalid source topology',()=>{
 const a=box3d([0,0,0],[2,3,4]),b=box3d([-1,-1,-1],[1,1,1]),r=certifyMinkowski3d(a,b),proof={version:1 as const,a,b,certificate:r.certificate}
 for(const mutate of [(p:typeof proof)=>{p.a.positions[0]+=.5},(p:typeof proof)=>{p.certificate.vertices[0][0][0]='17'},(p:typeof proof)=>{p.certificate.triangles.pop()},(p:typeof proof)=>{p.certificate.sourceCells[0]++},(p:typeof proof)=>{(p.certificate as any).unknown=true},(p:typeof proof)=>{p.certificate.vertices[0][0][1]='0'}]){const bad=structuredClone(proof);mutate(bad);expect(()=>verifyMinkowski3d(bad)).toThrow()}
 const open=structuredClone(a);open.indices.pop();expect(()=>certifyMinkowski3d(open,b)).toThrow()
 const reversed=structuredClone(a);[reversed.indices[1],reversed.indices[2]]=[reversed.indices[2],reversed.indices[1]];expect(()=>certifyMinkowski3d(reversed,b)).toThrow()
})
it('adds a source-preserving body transaction and a standalone proof without labeling it B-rep',()=>{
 const a=box3d([0,0,0],[2,3,4]),b=box3d([-1,-1,-1],[1,1,1]),doc={version:1 as const,sketches:[],bodies:[{id:'a',name:'A',mesh:normalizePolygonMesh(structuredClone(a))},{id:'b',name:'B',mesh:normalizePolygonMesh(structuredClone(b))}]},before=structuredClone(doc)
 const r=applySolidMinkowski(doc,{ids:['a','b'],createdId:'sum',ru:true});expect(doc).toEqual(before);expect(r.document.bodies.slice(0,2)).toEqual(doc.bodies);expect(r.document.bodies[2].brep).toBeUndefined();expect(r.resultId).toBe('sum');expect(verifyMinkowski3d(JSON.parse(JSON.stringify(r.proof))).volumeMm3).toBe(120)
 const expected=mainSolidExpectation({kind:'minkowski3d',document:doc,options:{ids:['a','b'],createdId:'sum',ru:true}});expect(mainSolidResult(expected,r)).toBe(true);expect(mainSolidResult(expected,{...r,volumeMm3:NaN})).toBe(false);expect(mainSolidResult(expected,{...r,resultId:'a'})).toBe(false)
 expect(()=>applySolidMinkowski(doc,{ids:['a','a'],createdId:'sum',ru:true})).toThrow();expect(()=>applySolidMinkowski(doc,{ids:['a','b'],createdId:'a',ru:true})).toThrow();expect(doc).toEqual(before)
})
it('executes the real worker route, preserves source buffers and recovers from a refusal',async()=>{
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(r=>messages.push(r)),a=box3d([0,0,0],[2,3,4]),b=box3d([-1,-1,-1],[1,1,1]),doc={version:1 as const,sketches:[],bodies:[{id:'a',name:'A',mesh:normalizePolygonMesh(a)},{id:'b',name:'B',mesh:normalizePolygonMesh(b)}]}
 await handle({version:1,id:1,job:{kind:'minkowski3d',document:doc,options:{ids:['a','a'],createdId:'sum',ru:true}}});expect(messages[0]).toMatchObject({ok:false})
 await handle({version:1,id:2,job:{kind:'minkowski3d',document:doc,options:{ids:['a','b'],createdId:'sum',ru:true}}});expect(messages[1]).toMatchObject({ok:true,kind:'minkowski3d',result:{volumeMm3:120,resultId:'sum'}});expect(doc.bodies[0].mesh.positions.byteLength).toBeGreaterThan(0);expect(doc.bodies).toHaveLength(2)
})
