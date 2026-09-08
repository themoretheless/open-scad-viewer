import {describe,it,expect} from 'vitest'
import {buildTextNurbsScene} from '../src/services/modelGraphTextScene'
import {isGeometryEvaluationResultPayload} from '../src/services/geometryWorkerProtocol'
import {geometrySceneFromMeshes,meshesFromGeometryScene} from '../src/core/scene'
import {assertNativeReferenceCurrent,nativeFaceReference,isNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {remapNativeFaceSelection} from '../src/services/nativeFaceSelection'
import type {PickHit} from '../src/services/rendererContracts'
const surface={id:'patch',op:'surface',degree_u:1,degree_v:1,knots_u:[0,0,1,1],knots_v:[0,0,1,1],control_points:[[[0,0,0],[0,10,0]],[[10,0,0],[10,10,1.12345678912345]]],weights:[[1,1],[1,1]]}
const document={language:'modelgraph/nurbs-1',units:'mm',parameters:[],nodes:[surface],root:'patch'}
const hit:PickHit={meshIndex:0,triangleIndex:0,faceId:0,point:[0,0,0],normal:[0,0,1],barycentric:[1,0,0],source:null,backside:false}
describe('native authority survives scene publication',()=>{
 it('shows a native surface with independent display detail and preserves f64 source',()=>{
  const low=buildTextNurbsScene(document,'preview'),full=buildTextNurbsScene(document,'full')
  expect(low.meshes[0].indices.length).toBeLessThan(full.meshes[0].indices.length)
  const a=low.meshes[0].nativeGeometry!,b=full.meshes[0].nativeGeometry!
  expect(a.revision).toBe(b.revision);expect(a.documentRevision).toBe(b.documentRevision)
  expect(JSON.parse(a.geometryJson).geometry.controlPoints[1][1][2]).toBe(1.12345678912345)
  expect(low.meshes[0].entityId).toBe('entity:native/patch')
  expect(isGeometryEvaluationResultPayload(structuredClone(full))).toBe(true)
  const restored=meshesFromGeometryScene(geometrySceneFromMeshes(full.meshes))
  expect(restored[0].nativeGeometry).toEqual(b)
  expect(remapNativeFaceSelection(low.meshes[0],full.meshes[0],hit,0)?.faceId).toBe(0)
 })
 it('rejects a face reference after source geometry changes',()=>{
  const before=buildTextNurbsScene(document).meshes[0]
  const changed=structuredClone(document);changed.nodes[0].control_points[1][1][2]=2
  const after=buildTextNurbsScene(changed).meshes[0]
  expect(()=>assertNativeReferenceCurrent(after.nativeGeometry!,nativeFaceReference(before.nativeGeometry!,0))).toThrow(/stale/)
  expect(remapNativeFaceSelection(before,after,hit,0)).toBeNull()
 })
 it('preserves explicit trimming in the native revision without including sampling density',()=>{
  const make=(segments:number,size:number)=>({...document,nodes:[surface,{id:'display',op:'tessellate',input:'patch',segments_u:segments,segments_v:segments,trim:{outer:[[0,0],[size,0],[size,size],[0,size]],holes:[]}}],root:'display'})
  const first=buildTextNurbsScene(make(4,0.8)).meshes[0],fine=buildTextNurbsScene(make(12,0.8)).meshes[0],cut=buildTextNurbsScene(make(4,0.5)).meshes[0]
  expect(first.nativeGeometry!.revision).toBe(fine.nativeGeometry!.revision)
  expect(first.nativeGeometry!.revision).not.toBe(cut.nativeGeometry!.revision)
  expect(first.nativeGeometry!.nodeId).toBe('patch')
 })
 it('rejects mutated snapshots at the worker boundary',()=>{
  const result=buildTextNurbsScene(document)
  result.meshes[0].nativeGeometry={...result.meshes[0].nativeGeometry!,geometryJson:'{}'}
  expect(isGeometryEvaluationResultPayload(result)).toBe(false)
  expect(isNativeGeometryArtifact({...result.meshes[0].nativeGeometry,extra:true})).toBe(false)
 })
 it('shows a native B-rep while keeping its six source face IDs',()=>{
  const result=buildTextNurbsScene({language:'modelgraph/nurbs-1',units:'mm',parameters:[],nodes:[{id:'box',op:'brep_box',min:[0,0,0],max:[2,3,4]}],root:'box'},'preview')
  expect(result.meshes[0].nativeGeometry!.kind).toBe('brep')
  expect(new Set(result.meshes[0].faceIds).size).toBe(6)
  expect(result.volume).toBeCloseTo(24)
 })
})
