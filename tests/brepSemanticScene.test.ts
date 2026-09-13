import {describe, expect, it, vi} from 'vitest'
import {buildBrepSemanticScene} from '../src/services/brepSemanticScene'
import {lowerOpenSCADToSemanticProgram} from '../src/services/semanticProgramLowerer'
import {geometrySceneFromMeshes, meshesFromGeometryScene} from '../src/core/scene'
import {isGeometryEvaluationResultPayload} from '../src/services/geometryWorkerProtocol'
import {inspectNurbsBrep} from '../src/services/geometry/brep'
import * as nativeExecutor from '../src/services/brepNativeExecutor'
import {assertNativeReferenceCurrent, nativeFaceReference} from '../src/core/nativeGeometry'

const header = '// @language openscad-viewer/brep-1\n'
const lower = (source: string) => lowerOpenSCADToSemanticProgram(header + source)
const policy = {quality: 'full' as const, segments: 8}

describe('real B-rep SemanticProgram scene assembly', () => {
  it('displays a partial hollow torus with two holed caps and one shell', async () => {
    const build = await buildBrepSemanticScene(lower('rotate_extrude(angle=90) translate([3,0,0]) difference(){circle(1);circle(0.5);}'), policy)
    const geometry = JSON.parse(build.result.meshes[0].nativeGeometry!.geometryJson).geometry
    expect(geometry.shells).toHaveLength(1)
    expect(geometry.bodies[0].innerShells).toHaveLength(0)
    expect(geometry.faces.filter((face:{holes:number[]})=>face.holes.length===1)).toHaveLength(2)
    expect(inspectNurbsBrep(geometry).topologyValid).toBe(true)
    expect(Math.abs(build.result.volume/(1.125*Math.PI**2)-1)).toBeLessThan(0.05)
  })
  it('retains the cavity shell of a fully revolved annular profile', async () => {
    const build = await buildBrepSemanticScene(lower('rotate_extrude() translate([3,0,0]) difference(){circle(1);circle(0.5);}'), policy)
    const geometry = JSON.parse(build.result.meshes[0].nativeGeometry!.geometryJson).geometry
    expect(geometry.bodies).toHaveLength(1)
    expect(geometry.bodies[0].innerShells).toHaveLength(1)
    expect(geometry.shells).toHaveLength(2)
    expect(inspectNurbsBrep(geometry).topologyValid).toBe(true)
    expect(Math.abs(build.result.volume/(4.5*Math.PI**2)-1)).toBeLessThan(0.05)
  })
  it('revolves a clipped circular profile into a sphere with shared poles', async () => {
    const build = await buildBrepSemanticScene(lower('rotate_extrude() intersection(){circle(1);translate([0,-2]) square([2,4]);}'), policy)
    const geometry = JSON.parse(build.result.meshes[0].nativeGeometry!.geometryJson).geometry
    expect(inspectNurbsBrep(geometry).topologyValid).toBe(true)
    expect(geometry.vertices.filter((v: {point:number[]})=>v.point[0]===0 && v.point[1]===0)).toHaveLength(2)
    expect(Math.abs(build.result.volume/(4*Math.PI/3)-1)).toBeLessThan(0.05)
  })
  it('displays a capped curved revolution while retaining native faces', async () => {
    const build = await buildBrepSemanticScene(lower('rotate_extrude(angle=90) translate([3,0,0]) circle(1);'), policy)
    expect(build.result.meshes).toHaveLength(1)
    const mesh = build.result.meshes[0]
    const geometry = JSON.parse(mesh.nativeGeometry!.geometryJson).geometry
    expect(geometry.faces).toHaveLength(6)
    expect(inspectNurbsBrep(geometry).topologyValid).toBe(true)
    expect(mesh.faceIdsAuthoritative).toBe(true)
    expect(build.result.volume).toBeGreaterThan(0)
    expect(Math.abs(build.result.volume / (1.5*Math.PI**2) - 1)).toBeLessThan(0.05)
  })
  it('retains distinct colored occurrences, authoritative faces and f64 snapshots through scene transport', async () => {
    const input = lower('color("red") translate([1,2,3]) cube([2,3,4]); color("blue") translate([8,0,0]) sphere(2);')
    const build = await buildBrepSemanticScene(input, policy)
    expect(build.result.meshes).toHaveLength(2)
    expect(build.outputs).toHaveLength(2)
    expect(new Set(build.outputs.map(o => o.entityId)).size).toBe(2)
    expect(build.result.meshes.map(m => m.color)).toEqual([[1,0,0,1], [0,0,1,1]])
    expect(build.result.meshes[0].provenance[0].source?.label).toBe('color()')
    expect(build.result.meshes[0].entityId).toBe(build.outputs[0].entityId)
    expect(isGeometryEvaluationResultPayload(structuredClone(build.result))).toBe(true)
    const restored = meshesFromGeometryScene(geometrySceneFromMeshes(build.result.meshes))
    for (let i = 0; i < restored.length; i++) {
      expect(restored[i].nativeGeometry).toEqual(build.result.meshes[i].nativeGeometry)
      const snapshot = restored[i].nativeGeometry!
      expect(inspectNurbsBrep(JSON.parse(snapshot.geometryJson).geometry).topologyValid).toBe(true)
      expect(JSON.parse(snapshot.documentJson).core.language.contract).toBe('openscad-viewer/brep-1')
      expect(restored[i].faceIdsAuthoritative).toBe(true)
      expect(build.outputs[i].topologyFaceIds).not.toHaveLength(0)
    }
    const cube = JSON.parse(restored[0].nativeGeometry!.geometryJson).geometry
    expect(cube.vertices.some((v: {point: number[]}) => JSON.stringify(v.point) === '[1,2,3]')).toBe(true)
  })

  it('separates source, semantic, requested display, actual display and native geometry identities', async () => {
    const input = lower('sphere(2);')
    const low = await buildBrepSemanticScene(input, {quality: 'preview', segments: 4})
    const fine = await buildBrepSemanticScene(input, {quality: 'full', segments: 12})
    expect(low.attestation).toEqual(fine.attestation)
    expect(low.displayPolicyHash).not.toBe(fine.displayPolicyHash)
    expect(low.outputs[0].snapshotRevision).toBe(fine.outputs[0].snapshotRevision)
    expect(low.outputs[0].entityId).toBe(fine.outputs[0].entityId)
    expect(low.outputs[0].topologyFaceIds).toEqual(fine.outputs[0].topologyFaceIds)
    expect(low.outputs[0].geometryAssetId).not.toBe(fine.outputs[0].geometryAssetId)
    expect(low.result.meshes[0].indices.length).toBeLessThan(fine.result.meshes[0].indices.length)
    const authored = await buildBrepSemanticScene(lower('$fn=64;sphere(2);'), policy)
    const other = await buildBrepSemanticScene(lower('$fn=8;sphere(2);'), policy)
    expect(authored.attestation.sourceHash).not.toBe(other.attestation.sourceHash)
    expect(authored.attestation.programHash).toBe(other.attestation.programHash)
    expect(authored.attestation.tessellationPolicyHash).not.toBe(other.attestation.tessellationPolicyHash)
    expect(authored.displayPolicyHash).toBe(other.displayPolicyHash)
    expect(authored.outputs[0].snapshotRevision).toBe(other.outputs[0].snapshotRevision)
    expect(authored.deviationStatus).toBe('not_certified')
    expect(authored.metrics).toBe('display_mesh_estimates')
  })

  it('publishes canonical empty results without meshes, phantom metrics or warnings', async () => {
    const build = await buildBrepSemanticScene(lower('difference(){cube(2);cube(2);}'), policy)
    expect(build.result.meshes).toEqual([])
    expect(build.result.volume).toBe(0)
    expect(build.result.surfaceArea).toBe(0)
    expect(build.result.warnings).toEqual([])
    expect(build.outputs).toHaveLength(1)
    expect(build.outputs[0]).toMatchObject({empty: true, snapshotRevision: null, geometryAssetId: null})
    const noOutputs = await buildBrepSemanticScene(lower(''), policy)
    expect(noOutputs.outputs).toEqual([])
  })

  it('fails atomically on cancellation, unsupported nodes and invalid display policy', async () => {
    const controller = new AbortController()
    controller.abort()
    await expect(buildBrepSemanticScene(lower('cube(1);'), policy, {signal: controller.signal})).rejects.toMatchObject({code: 'E_SEMANTIC_ABORTED'})
    await expect(buildBrepSemanticScene(lower('cube(1);hull(){sphere(1);translate([1,0,0])sphere(1);}'), policy)).rejects.toMatchObject({code: 'E_SEMANTIC_BACKEND_FAILURE'})
    await expect(buildBrepSemanticScene(lower('cube(1);'), {...policy, segments: 33})).rejects.toThrow('1..32')
    const recovered = await buildBrepSemanticScene(lower('cube(2);'), policy)
    expect(recovered.result.volume).toBeCloseTo(8)
  })

  it('enforces the display triangle budget across all individually valid outputs', async () => {
    const display = {quality: 'full' as const, segments: 12}
    const single = await buildBrepSemanticScene(lower('sphere(1);'), display)
    const count = Math.floor(20_000 / (single.result.meshes[0].indices.length / 3)) + 1
    await expect(buildBrepSemanticScene(lower(`for(i=[0:${count - 1}])translate([i*3,0,0])sphere(1);`), display))
      .rejects.toThrow('20000 display triangles')
  }, 20_000)

  it('refuses f64 geometry that collapses in the Float32 display representation', async () => {
    await expect(buildBrepSemanticScene(lower('translate([1e5,0,0])cube(0.001);'), policy))
      .rejects.toThrow(/Float32/)
  })

  it('refuses newly merged shells even when their individual display triangles stay nondegenerate', async () => {
    await expect(buildBrepSemanticScene(lower('union(){translate([999990,0,0])cube(1);translate([999991.01,0,0])cube(1);}'), {...policy, segments: 1}))
      .rejects.toThrow(/merge in Float32/)
  })

  it('binds native selections to semantic occurrence identity across unrelated equal geometry', async () => {
    const first = (await buildBrepSemanticScene(lower('cube(1);'), policy)).result.meshes[0]
    const second = (await buildBrepSemanticScene(lower('module changed(){cube(1);}changed();'), policy)).result.meshes[0]
    expect(first.entityId).not.toBe(second.entityId)
    expect(first.nativeGeometry!.revision).toBe(second.nativeGeometry!.revision)
    expect(() => assertNativeReferenceCurrent(second.nativeGeometry!, nativeFaceReference(first.nativeGeometry!, 0))).toThrow(/stale/)
  })

  it('does not publish if cancellation arrives while releasing committed native snapshots', async () => {
    const controller = new AbortController()
    const execute = nativeExecutor.executeBrepNativeProgram
    const spy = vi.spyOn(nativeExecutor, 'executeBrepNativeProgram').mockImplementation(async (input, control) => {
      const result = await execute(input, control)
      return {...result, dispose: async () => {
        await result.dispose()
        controller.abort()
      }}
    })
    try {
      await expect(buildBrepSemanticScene(lower('cube(1);'), policy, {signal: controller.signal}))
        .rejects.toMatchObject({code: 'E_SEMANTIC_ABORTED'})
    } finally { spy.mockRestore() }
  })
})
