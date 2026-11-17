import { describe, expect, it } from 'vitest'
import { measureBodyProperties } from '../src/app/bodyProperties'
import { encodeGlb, GLB_ROOT_MATRIX, validateGlb, type GlbMesh } from '../src/exchange/glbExport'
import {
  ISO_PITCH,
  ISO_YAW,
  standardViewOrientation,
  VIEW_CUBE_TARGETS,
  viewCubeHotspots,
  viewCubeOrientation,
  viewCubeTargetActive,
} from '../src/render/viewportModel'
import { RecordingRenderer } from '../src/render/recordingRenderer'

const IDENTITY = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])

/** Axis-aligned box as a shared-vertex triangle mesh (8 vertices, 12 outward triangles, 6 face ids). */
function box(size: [number, number, number], offset: [number, number, number] = [0, 0, 0]) {
  const corners: number[] = []
  for (let i = 0; i < 8; i++) {
    const p = [i & 1 ? size[0] : 0, i & 2 ? size[1] : 0, i & 4 ? size[2] : 0]
    corners.push(p[0]! + offset[0], p[1]! + offset[1], p[2]! + offset[2], 0, 0, 1)
  }
  const quads = [
    [0, 2, 3, 1],
    [4, 5, 7, 6],
    [0, 1, 5, 4],
    [2, 6, 7, 3],
    [0, 4, 6, 2],
    [1, 3, 7, 5],
  ]
  const indices = quads.flatMap(([a, b, c, d]) => [a!, b!, c!, a!, c!, d!])
  return {
    vertices: new Float32Array(corners),
    indices: new Uint32Array(indices),
    transform: IDENTITY,
    faceIds: new Uint32Array(quads.flatMap((_, face) => [face, face])),
    color: [1, 0.5, 0, 1] as [number, number, number, number],
  }
}

describe('P10 ViewCube targets', () => {
  it('has 6 faces, 12 edges and 8 corners with unique ids and bilingual labels', () => {
    expect(VIEW_CUBE_TARGETS).toHaveLength(26)
    const kinds = VIEW_CUBE_TARGETS.reduce<Record<string, number>>(
      (acc, t) => ({ ...acc, [t.kind]: (acc[t.kind] ?? 0) + 1 }),
      {},
    )
    expect(kinds).toEqual({ face: 6, edge: 12, corner: 8 })
    expect(new Set(VIEW_CUBE_TARGETS.map(t => t.id)).size).toBe(26)
    for (const t of VIEW_CUBE_TARGETS)
      expect(t.label).toMatch(/^[A-Z][a-z ]+ (face|edge|corner) \/ (Грань|Ребро|Угол): /)
  })

  it('agrees with the standard presets and the isometric corner', () => {
    for (const view of ['front', 'back', 'left', 'right', 'top', 'bottom'] as const) {
      const target = VIEW_CUBE_TARGETS.find(t => t.id === view)!
      const [yaw, pitch] = standardViewOrientation(view)
      expect(viewCubeTargetActive(target, yaw, pitch), view).toBe(true)
    }
    const iso = VIEW_CUBE_TARGETS.find(t => t.id === 'top-front-right')!
    expect(iso.yaw).toBeCloseTo(ISO_YAW, 12)
    expect(iso.pitch).toBeCloseTo(ISO_PITCH, 12)
    const edge = VIEW_CUBE_TARGETS.find(t => t.id === 'top-back')!
    expect(edge.yaw).toBeCloseTo(Math.PI, 12)
    expect(edge.pitch).toBeCloseTo(Math.PI / 4, 12)
    expect(viewCubeOrientation([0, 0, -1])).toEqual({ yaw: 0, pitch: -Math.PI / 2 })
  })

  it('cuts the three drawn faces into 9 edge and 7 corner hotspots', () => {
    const hotspots = viewCubeHotspots()
    expect(hotspots.filter(h => h.target.kind === 'edge')).toHaveLength(9)
    expect(hotspots.filter(h => h.target.kind === 'corner')).toHaveLength(7)
    expect(hotspots.find(h => h.target.id === 'top-front-right')!.polygons).toHaveLength(3)
    expect(hotspots.find(h => h.target.id === 'top-front')!.polygons).toHaveLength(2)
    expect(hotspots.find(h => h.target.id === 'top-back')!.polygons).toHaveLength(1)
  })

  it('drives the recording render seam camera', () => {
    const renderer = new RecordingRenderer()
    const target = VIEW_CUBE_TARGETS.find(t => t.id === 'bottom-back-left')!
    renderer.setOrbitAngles(target.yaw, target.pitch)
    const camera = renderer.getCameraState()
    expect(viewCubeTargetActive(target, camera.yaw, camera.pitch)).toBe(true)
  })
})

describe('P10 body properties', () => {
  it('measures a closed box in world coordinates', () => {
    const mesh = box([2, 3, 4])
    const transform = new Float32Array([1, 0, 0, 10, 0, 1, 0, -5, 0, 0, 1, 1, 0, 0, 0, 1])
    const properties = measureBodyProperties({ ...mesh, transform })
    expect(properties.surfaceArea).toBeCloseTo(52, 9)
    expect(properties.volume).toBeCloseTo(24, 9)
    expect(properties.centroid![0]).toBeCloseTo(11, 9)
    expect(properties.centroid![1]).toBeCloseTo(-3.5, 9)
    expect(properties.centroid![2]).toBeCloseTo(3, 9)
    expect(properties.bounds).toEqual({ min: [10, -5, 1], max: [12, -2, 5] })
    expect(properties).toMatchObject({
      triangleCount: 12,
      vertexCount: 8,
      edgeCount: 18,
      faceCount: 6,
      closed: true,
      boundaryEdgeCount: 0,
      eulerCharacteristic: 2,
    })
  })

  it('reports an open surface with boundary edges', () => {
    const mesh = box([1, 1, 1])
    const open = { ...mesh, indices: mesh.indices.slice(6), faceIds: mesh.faceIds.slice(2) }
    const properties = measureBodyProperties(open)
    expect(properties.closed).toBe(false)
    expect(properties.boundaryEdgeCount).toBe(4)
    expect(properties.surfaceArea).toBeCloseTo(5, 9)
    expect(properties.faceCount).toBe(5)
  })
})

/** Re-pack a GLB from (possibly edited) JSON and BIN to exercise the validator. */
function pack(json: unknown, bin: Uint8Array | null, jsonPad = 0x20) {
  const raw = new TextEncoder().encode(JSON.stringify(json))
  const jsonLength = (raw.length + 3) & ~3
  const binLength = bin ? (bin.length + 3) & ~3 : 0
  const total = 20 + jsonLength + (bin ? 8 + binLength : 0)
  const out = new Uint8Array(total)
  const view = new DataView(out.buffer)
  view.setUint32(0, 0x46546c67, true)
  view.setUint32(4, 2, true)
  view.setUint32(8, total, true)
  view.setUint32(12, jsonLength, true)
  view.setUint32(16, 0x4e4f534a, true)
  out.set(raw, 20)
  out.fill(jsonPad, 20 + raw.length, 20 + jsonLength)
  if (bin) {
    view.setUint32(20 + jsonLength, binLength, true)
    view.setUint32(24 + jsonLength, 0x004e4942, true)
    out.set(bin, 28 + jsonLength)
  }
  return out
}

describe('P10 GLB export', () => {
  const red: GlbMesh = { ...box([2, 3, 4]), color: [1, 0, 0, 1], entityId: 'entity:a' }
  const glass: GlbMesh = {
    ...box([1, 1, 1], [5, 0, 0]),
    color: [0, 0, 1, 1],
    material: {
      id: 'm',
      name: 'Glass',
      baseColor: [0.5, 0.5, 0.5],
      metallic: 0.25,
      roughness: 0.1,
      emissive: [0, 0, 0],
      shadingModel: 'pbr',
      alpha: 0.4,
    },
    transform: new Float32Array([1, 0, 0, 3, 0, 1, 0, 4, 0, 0, 1, 5, 0, 0, 0, 1]),
  }

  it('writes a byte-exact GLB 2.0 container', () => {
    const bytes = encodeGlb([red, glass])
    const view = new DataView(bytes.buffer)
    expect(view.getUint32(0, true)).toBe(0x46546c67)
    expect(String.fromCharCode(...bytes.subarray(0, 4))).toBe('glTF')
    expect(view.getUint32(4, true)).toBe(2)
    expect(view.getUint32(8, true)).toBe(bytes.byteLength)
    expect(bytes.byteLength % 4).toBe(0)
    const jsonLength = view.getUint32(12, true)
    expect(jsonLength % 4).toBe(0)
    expect(String.fromCharCode(...bytes.subarray(16, 20))).toBe('JSON')
    const binHeader = 20 + jsonLength
    expect(binHeader % 4).toBe(0)
    const binLength = view.getUint32(binHeader, true)
    expect(binLength % 4).toBe(0)
    expect([...bytes.subarray(binHeader + 4, binHeader + 8)]).toEqual([0x42, 0x49, 0x4e, 0])
    expect(binHeader + 8 + binLength).toBe(bytes.byteLength)
    // Box: 8 vertices -> 96 B positions + 96 B normals + 36 indices -> 144 B, per mesh.
    expect(binLength).toBe(2 * (96 + 96 + 144))

    const result = validateGlb(bytes)
    expect(result.errors).toEqual([])
    const json = result.json!
    expect(json.asset.version).toBe('2.0')
    expect(json.buffers).toEqual([{ byteLength: binLength }])
    expect(json.nodes[0].matrix).toEqual([...GLB_ROOT_MATRIX])
    expect(json.nodes[0].children).toEqual([1, 2])
    expect(json.nodes[1].matrix).toBeUndefined()
    expect(json.nodes[2].matrix).toEqual([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 3, 4, 5, 1])
    expect(json.nodes[1].name).toBe('entity:a')
    for (const bv of json.bufferViews) expect(bv.byteOffset % 4).toBe(0)
    const position = json.accessors[json.meshes[0].primitives[0].attributes.POSITION]
    expect(position).toMatchObject({ componentType: 5126, count: 8, type: 'VEC3', min: [0, 0, 0], max: [2, 3, 4] })
    const indices = json.accessors[json.meshes[0].primitives[0].indices]
    expect(indices).toMatchObject({ componentType: 5125, count: 36, type: 'SCALAR', min: [0], max: [7] })
    const positions = new Float32Array(result.bin!.slice(0, 96).buffer)
    expect([...positions.slice(21, 24)]).toEqual([2, 3, 4])
  })

  it('carries colours and materials as linear PBR factors', () => {
    const json = validateGlb(encodeGlb([red, glass, { ...red, entityId: 'entity:b' }])).json!
    expect(json.materials).toHaveLength(2)
    expect(json.materials[0].pbrMetallicRoughness).toEqual({
      baseColorFactor: [1, 0, 0, 1],
      metallicFactor: 0,
      roughnessFactor: 1,
    })
    expect(json.materials[0].alphaMode).toBeUndefined()
    expect(json.materials[1].name).toBe('Glass')
    expect(json.materials[1].alphaMode).toBe('BLEND')
    expect(json.materials[1].pbrMetallicRoughness.baseColorFactor[0]).toBeCloseTo(0.214041, 5)
    expect(json.materials[1].pbrMetallicRoughness.baseColorFactor[3]).toBeCloseTo(0.4, 6)
    expect(json.materials[1].pbrMetallicRoughness.metallicFactor).toBe(0.25)
    expect(json.meshes[2].primitives[0].material).toBe(0)
  })

  it('writes a valid empty scene without a BIN chunk', () => {
    const bytes = encodeGlb([])
    const result = validateGlb(bytes)
    expect(result.errors).toEqual([])
    expect(result.bin).toBeNull()
    expect(result.json!.buffers).toBeUndefined()
  })

  it('rejects broken headers, misaligned chunks and out-of-bounds accessors', () => {
    const bytes = encodeGlb([red])
    const good = validateGlb(bytes)
    const json = good.json!
    const bin = good.bin!

    const badMagic = bytes.slice()
    badMagic[0] = 0
    expect(validateGlb(badMagic).errors).toContain('bad magic')
    const badLength = bytes.slice()
    new DataView(badLength.buffer).setUint32(8, bytes.byteLength + 4, true)
    expect(validateGlb(badLength).errors).toContain('header length does not match file length')
    expect(validateGlb(pack(json, bin, 0)).errors).toContain('JSON chunk padding must be spaces')

    const overrun = structuredClone(json)
    overrun.accessors[0].count = 9
    expect(validateGlb(pack(overrun, bin)).errors).toContain('accessor 0 exceeds its bufferView')
    const misaligned = structuredClone(json)
    misaligned.accessors[0].byteOffset = 2
    expect(validateGlb(pack(misaligned, bin)).errors).toContain('accessor 0 is not aligned to its component size')
    const wrongBounds = structuredClone(json)
    wrongBounds.accessors[0].max = [2, 3, 5]
    expect(validateGlb(pack(wrongBounds, bin)).errors).toContain('mesh 0 POSITION min/max mismatch')
    const viewOverrun = structuredClone(json)
    viewOverrun.bufferViews[2].byteLength += 4
    expect(validateGlb(pack(viewOverrun, bin)).errors).toContain('bufferView 2 exceeds its buffer')
    const badIndex = bin.slice()
    new DataView(badIndex.buffer).setUint32(192, 99, true)
    expect(validateGlb(pack(json, badIndex)).errors).toContain('mesh 0 index 99 out of range')
  })
})
