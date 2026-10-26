import {expect,it} from 'vitest'
import {prepareMainSolidTransfer} from '../src/services/mainSolidWorkerTransport'
import type {MainSolidResponse} from '../src/services/mainSolidProtocol'
import type {DirectDocument} from '../src/services/directModeling'
it.each(['restoreDocument','sceneEdit'] as const)('moves %s mesh buffers exactly once while preserving views, UVs and aliases',(kind)=>{
 const positions=new Float64Array([0,0,0,1,0,0,0,1,0]),indices=new Uint32Array([0,1,2]),uv=new Float64Array([0,0,1,0,0,1])
 const document:DirectDocument={version:1,sketches:[],bodies:[{id:'a',name:'A',mesh:{positions,indices,uv}},{id:'b',name:'B',mesh:{positions:positions.subarray(0),indices}}]}
 const response:MainSolidResponse={version:1,id:1,kind,ok:true,result:document}
 const prepared=prepareMainSolidTransfer(response)
 expect(prepared.transfer).toHaveLength(3)
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 expect(positions.byteLength).toBe(0);expect(indices.byteLength).toBe(0);expect(uv.byteLength).toBe(0)
 if(!received.ok)throw Error('Unexpected failure')
 const bodies=(received.result as DirectDocument).bodies
 expect(Array.from(bodies[0].mesh.positions)).toEqual([0,0,0,1,0,0,0,1,0])
 expect(Array.from(bodies[0].mesh.indices)).toEqual([0,1,2]);expect(Array.from(bodies[0].mesh.uv!)).toEqual([0,0,1,0,0,1])
 expect(bodies[1].mesh.positions.buffer).toBe(bodies[0].mesh.positions.buffer)
 expect(bodies[1].mesh.indices.buffer).toBe(bodies[0].mesh.indices.buffer)
})
it('does not transfer unrelated scene results or failed responses',()=>{
 const document:DirectDocument={version:1,sketches:[],bodies:[{id:'a',name:'A',mesh:{positions:new Float64Array([0,0,0]),indices:new Uint32Array()}}]}
 const response:MainSolidResponse={version:1,id:1,kind:'pointEdit',ok:true,result:document}
 expect(prepareMainSolidTransfer(response).transfer).toEqual([])
 expect(document.bodies[0].mesh.positions.byteLength).toBe(24)
 expect(prepareMainSolidTransfer({version:1,id:2,kind:'restoreDocument',ok:false,error:{name:'Error',message:'invalid'}}).transfer).toEqual([])
})

it('rejects out-of-range or malformed profile provenance without throwing',async()=>{
 const {mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const expected={kind:'profilePrepare' as const,chainSegments:[2,1]}
 const source={chain:0,segment:1,reversed:true,connector:false}
 const result={document:{version:1,sketches:[],bodies:[]},id:'a',plane:{origin:[0,0,0],u:[1,0,0],v:[0,1,0]},report:{accepted:false,reason:'invalid-contour',points:[],defects:[],connectors:[],curveSources:[source]}}
 expect(mainSolidResult(expected,result)).toBe(true)
 for(const invalid of [{...source,chain:2},{...source,segment:2},{...source,reversed:'yes'},{chain:0,end:'middle',nextChain:1,reversed:false,connector:true},{chain:0,end:'end',nextChain:2,reversed:false,connector:true}]){
  expect(mainSolidResult(expected,{...result,report:{...result.report,curveSources:[invalid]}})).toBe(false)
 }
 expect(mainSolidResult(expected,{...result,report:{...result.report,profile:{loops:null}}})).toBe(false)
})

it('moves every force-marker mesh buffer without detaching request geometry',async()=>{
 const {trussFieldMeshes}=await import('../src/services/trussFieldMeshes')
 const model:import('../src/services/trussAnalysis').TrussModel={nodesMm:[[0,0,0],[10,0,0]],members:[{nodes:[0,1],youngMpa:2000,areaMm2:2}],restrained:[[true,true,true],[false,true,true]],forcesN:[[0,0,0],[100,0,0]]}
 const result:import('../src/services/trussAnalysis').TrussResponse={displacementsMm:[[0,0,0],[.25,0,0]],reactionsN:[[-100,0,0],[0,0,0]],axialForcesN:[100],axialStressesMpa:[50],maxDeflectionMm:.25,maxRelativeResidual:0,freeDofs:1}
 const meshes=trussFieldMeshes(model,result,1),expected=Array.from(meshes[0].vertices),modelBefore=structuredClone(model)
 const response:MainSolidResponse={version:1,id:3,kind:'trussField',ok:true,result:meshes}
 const prepared=prepareMainSolidTransfer(response)
 expect(prepared.transfer).toHaveLength(8)
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 expect(meshes[0].vertices.byteLength).toBe(0);expect(meshes[0].bvh.bounds.byteLength).toBe(0)
 if(!received.ok)throw Error('Unexpected failure')
 const moved=received.result as import('../src/core/mesh').MeshData[]
 expect(Array.from(moved[0].vertices)).toEqual(expected);expect(model).toEqual(modelBefore)
})

it('moves batch manifold reports without detaching source mesh buffers',async()=>{
 const {inspectMeshManifoldObjects}=await import('../src/services/meshManifoldDefects')
 const source={positions:new Float64Array([0,0,0,2,0,0,0,2,0,0,0,2]),indices:new Uint32Array([0,2,1,0,1,3,1,2,3])}
 const result=inspectMeshManifoldObjects([{id:'open',mesh:source}])
 const prepared=prepareMainSolidTransfer({version:1,id:4,kind:'meshManifold',ok:true,result})
 expect(prepared.transfer).toContain(result[0].report.boundaryEdges.buffer)
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 if(!received.ok)throw Error('Unexpected failure')
 const moved=received.result as import('../src/services/meshManifoldDefects').MeshManifoldEntry[]
 expect(moved[0].id).toBe('open');expect(moved[0].report.boundaryEdges).toHaveLength(6)
 expect(result[0].report.boundaryEdges.byteLength).toBe(0)
 expect(source.positions.byteLength).toBe(96);expect(source.indices.byteLength).toBe(36)
})

it('moves repaired mesh and diagnostic buffers without copying their contents',async()=>{
 const {repairMeshObject}=await import('../src/services/meshEditing')
 const source={positions:new Float64Array([0,0,0,2,0,0,0,2,0,0,0,2]),indices:new Uint32Array([0,2,1,0,1,3,1,2,3])}
 const result=repairMeshObject(source),expected=Array.from(result.mesh.indices)
 const prepared=prepareMainSolidTransfer({version:1,id:4,kind:'meshRepair',ok:true,result})
 expect(prepared.transfer).toContain(result.mesh.positions.buffer);expect(prepared.transfer).toContain(result.before.boundaryEdges.buffer)
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 if(!received.ok)throw Error('Unexpected failure')
 const moved=received.result as import('../src/services/meshEditing').MeshRepairResult
 expect(Array.from(moved.mesh.indices)).toEqual(expected);expect(moved.after.isManifold).toBe(true)
 expect(result.mesh.indices.byteLength).toBe(0);expect(source.indices.byteLength).toBe(36)
})

it('moves manifold Boolean output without detaching either operand',async()=>{
 const {booleanMeshObjectsDetailed,createBoxMesh,transformMesh}=await import('../src/services/meshEditing')
 const a=createBoxMesh([2,2,2]),b=transformMesh(createBoxMesh([2,2,2]),[1,0,0],0,1)
 const result=booleanMeshObjectsDetailed(a,b,'union'),expected=Array.from(result.mesh.positions)
 const prepared=prepareMainSolidTransfer({version:1,id:5,kind:'meshBoolean',ok:true,result})
 expect(prepared.transfer).toEqual(expect.arrayContaining([result.mesh.positions.buffer,result.mesh.indices.buffer]))
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 if(!received.ok)throw Error('Unexpected failure')
 const moved=received.result as import('../src/services/meshEditing').MeshBooleanResult
 expect(Array.from(moved.mesh.positions)).toEqual(expected);expect(moved.report.isManifold).toBe(true)
 expect(result.mesh.positions.byteLength).toBe(0)
 expect(a.positions.byteLength).toBeGreaterThan(0);expect(b.positions.byteLength).toBeGreaterThan(0)
})

it('moves imported mesh and manifold diagnostics without detaching source bytes',async()=>{
 const {importMeshFile}=await import('../src/services/meshImport')
 const source=['v 0 0 0','v 1 0 0','v 0 1 0','v 0 0 1','f 1 3 2','f 1 2 4','f 2 3 4'].join('\n')
 const input=new TextEncoder().encode(source),result=await importMeshFile('open.obj',input,{repair:'full'})
 const prepared=prepareMainSolidTransfer({version:1,id:6,kind:'meshImport',ok:true,result})
 expect(prepared.transfer).toEqual(expect.arrayContaining([result.positions.buffer,result.indices.buffer,result.manifold.before.boundaryEdges.buffer]))
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 if(!received.ok)throw Error('Unexpected failure')
 const moved=received.result as import('../src/services/meshImport').ImportedMesh
 expect(moved.triangleCount).toBe(4);expect(moved.manifold.after.isManifold).toBe(true)
 expect(result.positions.byteLength).toBe(0);expect(input.byteLength).toBeGreaterThan(0)
})

it('moves converted bytes without detaching the source file buffer',async()=>{
 const {convertMeshFile}=await import('../src/services/meshConvert')
 const source=['v 0 0 0','v 1 0 0','v 0 1 0','v 0 0 1','f 1 3 2','f 1 2 4','f 2 3 4','f 1 4 3'].join('\n')
 const input=new TextEncoder().encode(source),result=await convertMeshFile('solid.obj',input,'ply'),expected=Array.from(result.data)
 const prepared=prepareMainSolidTransfer({version:1,id:7,kind:'meshConvert',ok:true,result})
 expect(prepared.transfer).toEqual([result.data.buffer])
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 if(!received.ok)throw Error('Unexpected failure')
 const moved=received.result as import('../src/services/meshConvert').MeshConvertResult
 expect(Array.from(moved.data)).toEqual(expected);expect(moved.fileName).toBe('solid.ply')
 expect(result.data.byteLength).toBe(0);expect(input.byteLength).toBeGreaterThan(0)
})

it('moves exported bytes without detaching the source mesh',async()=>{
 const {createBoxMesh}=await import('../src/services/meshEditing')
 const {exportPolygonMesh}=await import('../src/services/meshConvert')
 const mesh=createBoxMesh([2,2,2]),result=await exportPolygonMesh(mesh,'ply'),expected=Array.from(result.data)
 const prepared=prepareMainSolidTransfer({version:1,id:8,kind:'meshExport',ok:true,result})
 expect(prepared.transfer).toEqual([result.data.buffer])
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 if(!received.ok)throw Error('Unexpected failure')
 const moved=received.result as import('../src/services/meshConvert').MeshExportArtifact
 expect(Array.from(moved.data)).toEqual(expected);expect(moved.extension).toBe('ply')
 expect(result.data.byteLength).toBe(0)
 expect(mesh.positions.byteLength).toBeGreaterThan(0);expect(mesh.indices.byteLength).toBeGreaterThan(0)
})
