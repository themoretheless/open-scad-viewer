import {describe,expect,it} from 'vitest'
import {buildBrepSemanticScene} from '../src/services/brepSemanticScene'
import {lowerOpenSCADToSemanticProgram} from '../src/language/semantic/semanticProgramLowerer'
import {editExactFullRoundSceneMesh} from '../src/services/exactFullRoundHistoryScene'
import {restoreExactFullRoundSnapshot,createExactFullRoundSnapshot} from '../src/services/exactFullRoundSnapshot'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {assertNativeReferenceCurrent,nativeFaceReference} from '../src/core/nativeGeometry'
import {geometrySceneFromMeshes,meshesFromGeometryScene} from '../src/core/scene'
const build=()=>buildBrepSemanticScene(lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\nfull_round_exact([5,4,3]) cube([4,8,6]);'),{quality:'preview',segments:4})
describe('exact feature history scene publication on installed WASM',()=>{
 it('rebuilds the displayed body and persists undo/redo through scene serialization',async()=>{
  const original=(await build()).result.meshes[0]
  const before=structuredClone(original)
  const artifact=restoreExactFullRoundSnapshot(original.nativeGeometry!).artifact
  const source=callGeometryRust<any>('brep_nurbs_transform',{model:artifact.source,matrix:[[1,0,0,0],[0,1,0,0],[0,0,1.25,0],[0,0,0,1]]})
  const edited=editExactFullRoundSceneMesh(original,{action:'rebase-source',source},4)
  expect(edited.entityId).toBe(original.entityId)
  expect(edited.geometryAssetId).not.toBe(original.geometryAssetId)
  expect(edited.nativeGeometry!.revision).not.toBe(original.nativeGeometry!.revision)
  expect(()=>assertNativeReferenceCurrent(edited.nativeGeometry!,nativeFaceReference(original.nativeGeometry!,0))).toThrow(/stale/)
  expect(original).toEqual(before)
  const transported=meshesFromGeometryScene(geometrySceneFromMeshes([edited]))[0]
  const undone=editExactFullRoundSceneMesh(transported,{action:'undo'},4)
  expect(undone.geometryAssetId).toBe(original.geometryAssetId)
  expect(restoreExactFullRoundSnapshot(undone.nativeGeometry!).artifact).toEqual(artifact)
  const redone=editExactFullRoundSceneMesh(undone,{action:'redo'},8)
  expect(restoreExactFullRoundSnapshot(redone.nativeGeometry!).artifact).toEqual(restoreExactFullRoundSnapshot(edited.nativeGeometry!).artifact)
  expect(redone.indices.length).toBeGreaterThan(edited.indices.length)
  expect(redone.nativeGeometry!.revision).toBe(edited.nativeGeometry!.revision)
  expect(redone.provenance[0].triangleEnd).toBe(redone.indices.length/3)
 },30000)
 it('refuses a valid history attached to the wrong current body and leaves input intact',async()=>{
  const original=(await build()).result.meshes[0]
  const seeded=editExactFullRoundSceneMesh(original,{action:'restore'},4)
  const artifact=restoreExactFullRoundSnapshot(original.nativeGeometry!).artifact
  const source=callGeometryRust<any>('brep_nurbs_transform',{model:artifact.source,matrix:[[1,0,0,0],[0,1,0,0],[0,0,1.25,0],[0,0,0,1]]})
  const edited=editExactFullRoundSceneMesh(seeded,{action:'rebase-source',source},4)
  const wrong={...edited,nativeGeometry:createExactFullRoundSnapshot(edited.entityId!,artifact,JSON.parse(edited.nativeGeometry!.documentJson))}
  expect(()=>editExactFullRoundSceneMesh(wrong,{action:'restore'},4)).toThrow(/differs/)
  const before=structuredClone(seeded)
  expect(()=>editExactFullRoundSceneMesh(seeded,{action:'undo'},4)).toThrow()
  expect(seeded).toEqual(before)
 },30000)
})
