import { describe, expect, it, vi } from 'vitest'

// Optional qualification-only module delivery substitution. Without an external
// module these tests use the normal installed package. Every geometry call still
// executes the real Rust ABI; production delivery and worker bootstrap are not
// replaced or claimed by this test.
vi.mock('../src/kernel/geometry/kernelCompilation', async importOriginal => {
  const path = process.env.OWN_EXACT_FULL_ROUND_WASM
  if (!path) return importOriginal()
  const { readFileSync } = await import('node:fs')
  const { createHash } = await import('node:crypto')
  const receiptPath=process.env.OWN_EXACT_FULL_ROUND_RECEIPT ?? new URL('../docs/qualification/full-round-exact-scene-2026-11-02/module-identity.json', import.meta.url)
  const receipt = JSON.parse(readFileSync(receiptPath,'utf8'))
  const bytes = readFileSync(path)
  if (createHash('sha256').update(bytes).digest('hex') !== receipt.wasm_sha256) throw new Error('Unqualified exact WASM bytes')
  const module = new WebAssembly.Module(bytes)
  return { compileGeometryKernelArtifactSync: () => module, compileGeometryKernelArtifact: async () => module }
})
import * as nativeExecutor from '../src/kernel/brepNativeExecutor'
import { buildBrepSemanticScene } from '../src/kernel/brepSemanticScene'
import { lowerOpenSCADToSemanticProgram } from '../src/language/semantic/semanticProgramLowerer'
import { restoreExactFullRoundSnapshot } from '../src/kernel/exactFullRoundSnapshot'
import { isGeometryEvaluationResultPayload } from '../src/kernel/geometryWorkerProtocol'
import { geometrySceneFromMeshes, meshesFromGeometryScene } from '../src/core/scene'
import { remapNativeFaceSelection } from '../src/services/nativeFaceSelection'
import type { PickHit } from '../src/render/rendererContracts'
import { nativeFaceReference, assertNativeReferenceCurrent } from '../src/core/nativeGeometry'
const source = (height=6) => '// @language openscad-viewer/brep-1\nfull_round_exact([5,4,3]) cube([4,8,'+height+']);'
const build = (segments:number,height=6) => buildBrepSemanticScene(lowerOpenSCADToSemanticProgram(source(height)),{quality:'preview',segments})

describe('exact Full Round scene on qualified real WASM', () => {
  it('publishes display and exact snapshot with matching topology identity', async () => {
    const scene = await build(8)
    expect(scene.metrics).toBe('display_mesh_estimates')
    expect(scene.deviationStatus).toBe('not_certified')
    const mesh = scene.result.meshes[0], snapshot = mesh.nativeGeometry!
    expect(snapshot.kind).toBe('rational-brep')
    expect(mesh.faceIdsAuthoritative).toBe(true)
    expect(mesh.indices.length).toBeGreaterThan(0)
    expect(scene.outputs[0].snapshotRevision).toBe(snapshot.revision)
    const exact = restoreExactFullRoundSnapshot(snapshot)
    expect(scene.outputs[0].topologyFaceIds).toEqual((exact.model as any).topologyIds.faces)
    expect(new Set(mesh.faceIds).size).toBe(scene.outputs[0].topologyFaceIds.length)
    expect(typeof (exact.model as any).vertices[0].point[0][0]).toBe('string')
    expect(isGeometryEvaluationResultPayload(structuredClone(scene.result))).toBe(true)
  },30000)
  it('keeps exact geometry and entity stable across display detail changes', async () => {
    const low=await build(2), fine=await build(16)
    const a=low.result.meshes[0], b=fine.result.meshes[0]
    expect(a.indices.length).toBeLessThan(b.indices.length)
    expect(a.geometryAssetId).not.toBe(b.geometryAssetId)
    expect(a.entityId).toBe(b.entityId)
    expect(a.nativeGeometry).toEqual(b.nativeGeometry)
    expect(low.outputs[0].topologyFaceIds).toEqual(fine.outputs[0].topologyFaceIds)
    expect(low.displayPolicyHash).not.toBe(fine.displayPolicyHash)
    const triangle=a.faceIds.length-1
    const hit:PickHit={meshIndex:0,triangleIndex:triangle,faceId:a.faceIds[triangle],point:[0,0,0],normal:[0,0,1],barycentric:[1,0,0],source:null,backside:false}
    const remapped=remapNativeFaceSelection(a,b,hit,0)
    expect(remapped?.faceId).toBe(hit.faceId)
    expect(remapped?.barycentric).toEqual([1/3,1/3,1/3])
  },30000)
  it('retains the exact body after scene serialization and publication', async () => {
    const scene=await build(4)
    const original=scene.result.meshes[0]
    const transferred=meshesFromGeometryScene(structuredClone(geometrySceneFromMeshes(scene.result.meshes)))
    expect(transferred[0].nativeGeometry).toEqual(original.nativeGeometry)
    expect(restoreExactFullRoundSnapshot(transferred[0].nativeGeometry!).model)
      .toEqual(restoreExactFullRoundSnapshot(original.nativeGeometry!).model)
  },30000)
  it('invalidates geometry selections after a real source edit while keeping entity identity', async () => {
    const before=(await build(4)).result.meshes[0]
    const after=(await build(4,7)).result.meshes[0]
    expect(before.entityId).toBe(after.entityId)
    expect(before.nativeGeometry!.revision).not.toBe(after.nativeGeometry!.revision)
    expect(() => assertNativeReferenceCurrent(after.nativeGeometry!,nativeFaceReference(before.nativeGeometry!,0))).toThrow(/stale/)
    const hit:PickHit={meshIndex:0,triangleIndex:0,faceId:before.faceIds[0],point:[0,0,0],normal:[0,0,1],barycentric:[1,0,0],source:null,backside:false}
    expect(remapNativeFaceSelection(before,after,hit,0)).toBeNull()
  },30000)
  it('does not publish when cancellation arrives during exact result disposal', async () => {
    const controller=new AbortController(), execute=nativeExecutor.executeBrepNativeProgram
    const spy=vi.spyOn(nativeExecutor,'executeBrepNativeProgram').mockImplementation(async (input,control) => {
      const result=await execute(input,control)
      return {...result,dispose:async()=>{await result.dispose();controller.abort()}}
    })
    try {
      await expect(buildBrepSemanticScene(lowerOpenSCADToSemanticProgram(source()),{quality:'preview',segments:4},{signal:controller.signal}))
        .rejects.toMatchObject({code:'E_SEMANTIC_ABORTED'})
    } finally {spy.mockRestore()}
  },30000)
  it('aborts a computed exact node and permits a fresh successful evaluation', async () => {
    const controller=new AbortController()
    await expect(buildBrepSemanticScene(lowerOpenSCADToSemanticProgram(source()),{quality:'preview',segments:4},
      {signal:controller.signal,onNode:completed=>{if(completed===2)controller.abort()}}))
      .rejects.toMatchObject({code:'E_SEMANTIC_ABORTED'})
    expect((await build(4)).result.meshes[0].nativeGeometry!.kind).toBe('rational-brep')
  },30000)

  it('publishes two mapped exact outputs with distinct scene and snapshot ownership', async () => {
    const code='// @language openscad-viewer/brep-1\nfull_round_exact([5,4,3]) {cube([4,8,6]);cube([4,8,7]);}'
    const scene=await buildBrepSemanticScene(lowerOpenSCADToSemanticProgram(code),{quality:'preview',segments:4})
    expect(scene.result.meshes).toHaveLength(2)
    expect(scene.outputs).toHaveLength(2)
    expect(new Set(scene.result.meshes.map(mesh=>mesh.entityId)).size).toBe(2)
    expect(new Set(scene.result.meshes.map(mesh=>mesh.nativeGeometry!.revision)).size).toBe(2)
    for(let i=0;i<2;i++) {
      expect(scene.result.meshes[i].nativeGeometry!.nodeId).toBe(scene.outputs[i].entityId)
      expect(scene.result.meshes[i].nativeGeometry!.kind).toBe('rational-brep')
      expect(scene.outputs[i].snapshotRevision).toBe(scene.result.meshes[i].nativeGeometry!.revision)
    }
    expect(isGeometryEvaluationResultPayload(structuredClone(scene.result))).toBe(true)
  },30000)

})
