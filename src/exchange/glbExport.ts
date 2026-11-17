import type { MeshData } from '../core/mesh'

/** Binary glTF 2.0 (`.glb`) writer and structural validator for evaluated display meshes. */

const GLB_MAGIC = 0x46546c67 // 'glTF'
const GLB_VERSION = 2
const CHUNK_JSON = 0x4e4f534a // 'JSON'
const CHUNK_BIN = 0x004e4942 // 'BIN\0'
const VERTEX_STRIDE = 6

const FLOAT = 5126
const UNSIGNED_INT = 5125
const ARRAY_BUFFER = 34962
const ELEMENT_ARRAY_BUFFER = 34963
const TRIANGLES = 4

export type GlbMesh = Pick<MeshData, 'vertices' | 'indices' | 'transform' | 'color'> &
  Partial<Pick<MeshData, 'material' | 'entityId'>>

export interface GlbExportOptions {
  /** glTF asset.generator string. */
  generator?: string
  /** Node names, parallel to `meshes`. */
  names?: readonly string[]
}

const COMPONENTS: Record<string, number> = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 }
const COMPONENT_BYTES: Record<number, number> = { 5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4 }

const srgbToLinear = (value: number) => {
  const c = Math.min(1, Math.max(0, Number.isFinite(value) ? value : 0))
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}
const clamp01 = (value: number, fallback: number) =>
  Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : fallback
const pad4 = (length: number) => (length + 3) & ~3

/**
 * Root node transform: OpenSCAD millimetres, Z up -> glTF metres, Y up
 * (x, y, z) -> 0.001 · (x, z, -y), column-major.
 */
export const GLB_ROOT_MATRIX: readonly number[] = Object.freeze([
  0.001, 0, 0, 0, 0, 0, -0.001, 0, 0, 0.001, 0, 0, 0, 0, 0, 1,
])

/** Encode the given meshes (already filtered to the visible ones) as a single binary glTF. */
export function encodeGlb(meshes: readonly GlbMesh[], options: GlbExportOptions = {}): Uint8Array {
  const bufferViews: Array<Record<string, unknown>> = []
  const accessors: Array<Record<string, unknown>> = []
  const gltfMeshes: Array<Record<string, unknown>> = []
  const materials: Array<Record<string, unknown>> = []
  const materialIndex = new Map<string, number>()
  const nodes: Array<Record<string, unknown>> = [{ name: 'scene-root', matrix: [...GLB_ROOT_MATRIX], children: [] }]
  const parts: Uint8Array[] = []
  let byteLength = 0

  const addView = (bytes: Uint8Array, target: number) => {
    const offset = byteLength
    parts.push(bytes)
    byteLength += bytes.byteLength
    const padding = pad4(byteLength) - byteLength
    if (padding) {
      parts.push(new Uint8Array(padding))
      byteLength += padding
    }
    bufferViews.push({ buffer: 0, byteOffset: offset, byteLength: bytes.byteLength, target })
    return bufferViews.length - 1
  }

  const materialFor = (mesh: GlbMesh) => {
    const rgb = mesh.material?.baseColor ?? [mesh.color[0], mesh.color[1], mesh.color[2]]
    const alpha = clamp01(mesh.material?.alpha ?? mesh.color[3], 1)
    const metallic = clamp01(mesh.material?.metallic ?? 0, 0)
    const roughness = clamp01(mesh.material?.roughness ?? 1, 1)
    const emissive = mesh.material?.emissive ?? [0, 0, 0]
    const baseColorFactor = [...rgb.map(srgbToLinear), alpha].map(c => Math.round(c * 1e6) / 1e6)
    const emissiveFactor = emissive.map(srgbToLinear).map(c => Math.round(c * 1e6) / 1e6)
    const key = JSON.stringify([mesh.material?.name ?? '', baseColorFactor, metallic, roughness, emissiveFactor])
    let index = materialIndex.get(key)
    if (index === undefined) {
      const material: Record<string, unknown> = {
        name: mesh.material?.name ?? `color-${materials.length + 1}`,
        pbrMetallicRoughness: { baseColorFactor, metallicFactor: metallic, roughnessFactor: roughness },
        doubleSided: false,
      }
      if (emissiveFactor.some(c => c > 0)) material.emissiveFactor = emissiveFactor
      if (alpha < 1) material.alphaMode = 'BLEND'
      index = materials.length
      materials.push(material)
      materialIndex.set(key, index)
    }
    return index
  }

  meshes.forEach((mesh, meshIndex) => {
    const vertexCount = Math.floor(mesh.vertices.length / VERTEX_STRIDE)
    const triangleCount = Math.floor(mesh.indices.length / 3)
    if (!vertexCount || !triangleCount) return
    const positions = new Float32Array(vertexCount * 3)
    const normals = new Float32Array(vertexCount * 3)
    const min = [Infinity, Infinity, Infinity]
    const max = [-Infinity, -Infinity, -Infinity]
    for (let v = 0; v < vertexCount; v++) {
      for (let k = 0; k < 3; k++) {
        const value = mesh.vertices[v * VERTEX_STRIDE + k]!
        positions[v * 3 + k] = value
        // Bounds of the stored float32 values, as validators compare them exactly.
        const stored = positions[v * 3 + k]!
        if (stored < min[k]!) min[k] = stored
        if (stored > max[k]!) max[k] = stored
      }
      const nx = mesh.vertices[v * VERTEX_STRIDE + 3]!,
        ny = mesh.vertices[v * VERTEX_STRIDE + 4]!,
        nz = mesh.vertices[v * VERTEX_STRIDE + 5]!
      const length = Math.hypot(nx, ny, nz)
      if (length > 1e-12 && Number.isFinite(length)) {
        normals[v * 3] = nx / length
        normals[v * 3 + 1] = ny / length
        normals[v * 3 + 2] = nz / length
      } else normals[v * 3 + 2] = 1
    }
    const indices = new Uint32Array(triangleCount * 3)
    for (let i = 0; i < indices.length; i++) {
      const index = mesh.indices[i]!
      if (index >= vertexCount) throw new RangeError(`GLB export: mesh ${meshIndex} index ${index} out of range`)
      indices[i] = index
    }
    let maxIndex = 0,
      minIndex = Infinity
    for (const index of indices) {
      if (index > maxIndex) maxIndex = index
      if (index < minIndex) minIndex = index
    }

    const positionView = addView(new Uint8Array(positions.buffer), ARRAY_BUFFER)
    accessors.push({ bufferView: positionView, componentType: FLOAT, count: vertexCount, type: 'VEC3', min, max })
    const position = accessors.length - 1
    const normalView = addView(new Uint8Array(normals.buffer), ARRAY_BUFFER)
    accessors.push({ bufferView: normalView, componentType: FLOAT, count: vertexCount, type: 'VEC3' })
    const normal = accessors.length - 1
    const indexView = addView(new Uint8Array(indices.buffer), ELEMENT_ARRAY_BUFFER)
    accessors.push({
      bufferView: indexView,
      componentType: UNSIGNED_INT,
      count: indices.length,
      type: 'SCALAR',
      min: [minIndex],
      max: [maxIndex],
    })
    const indexAccessor = accessors.length - 1

    const name = options.names?.[meshIndex] ?? mesh.entityId ?? `body-${meshIndex + 1}`
    gltfMeshes.push({
      name,
      primitives: [
        {
          attributes: { POSITION: position, NORMAL: normal },
          indices: indexAccessor,
          material: materialFor(mesh),
          mode: TRIANGLES,
        },
      ],
    })
    const node: Record<string, unknown> = { name, mesh: gltfMeshes.length - 1 }
    const t = mesh.transform
    const identity = t.length !== 16 || t.every((value, i) => value === (i % 5 === 0 ? 1 : 0))
    if (!identity) {
      // Row-major affine -> glTF column-major.
      node.matrix = [0, 1, 2, 3].flatMap(column => [0, 1, 2, 3].map(row => t[row * 4 + column]!))
    }
    nodes.push(node)
    ;(nodes[0]!.children as number[]).push(nodes.length - 1)
  })

  const json: Record<string, unknown> = {
    asset: { version: '2.0', generator: options.generator ?? 'open-scad-viewer' },
    scene: 0,
    scenes: [{ name: 'Scene', nodes: [0] }],
    nodes,
  }
  if (gltfMeshes.length) {
    Object.assign(json, { meshes: gltfMeshes, materials, accessors, bufferViews, buffers: [{ byteLength }] })
  }
  if (!(nodes[0]!.children as number[]).length) delete nodes[0]!.children

  const jsonBytes = new TextEncoder().encode(JSON.stringify(json))
  const jsonLength = pad4(jsonBytes.byteLength)
  const binLength = byteLength ? pad4(byteLength) : 0
  const total = 12 + 8 + jsonLength + (binLength ? 8 + binLength : 0)
  const out = new Uint8Array(total)
  const view = new DataView(out.buffer)
  view.setUint32(0, GLB_MAGIC, true)
  view.setUint32(4, GLB_VERSION, true)
  view.setUint32(8, total, true)
  view.setUint32(12, jsonLength, true)
  view.setUint32(16, CHUNK_JSON, true)
  out.set(jsonBytes, 20)
  out.fill(0x20, 20 + jsonBytes.byteLength, 20 + jsonLength) // JSON pads with spaces.
  if (binLength) {
    const binStart = 20 + jsonLength
    view.setUint32(binStart, binLength, true)
    view.setUint32(binStart + 4, CHUNK_BIN, true)
    let at = binStart + 8
    for (const part of parts) {
      out.set(part, at)
      at += part.byteLength
    }
  }
  return out
}

export interface GlbValidation {
  ok: boolean
  errors: string[]
  json: Record<string, any> | null // eslint-disable-line @typescript-eslint/no-explicit-any -- parsed glTF JSON
  bin: Uint8Array | null
}

/**
 * Structural GLB check: header, chunk order, lengths and 4-byte alignment, JSON padding, buffer
 * size, buffer-view and accessor bounds and alignment, index ranges and POSITION min/max.
 */
export function validateGlb(bytes: Uint8Array): GlbValidation {
  const errors: string[] = []
  const fail = (message: string): GlbValidation => ({ ok: false, errors: [...errors, message], json: null, bin: null })
  if (bytes.byteLength < 20) return fail('file shorter than header + chunk header')
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  if (view.getUint32(0, true) !== GLB_MAGIC) return fail('bad magic')
  if (view.getUint32(4, true) !== GLB_VERSION) return fail('unsupported version')
  if (view.getUint32(8, true) !== bytes.byteLength) return fail('header length does not match file length')
  if (bytes.byteLength % 4) errors.push('file length is not 4-byte aligned')

  const jsonLength = view.getUint32(12, true)
  if (view.getUint32(16, true) !== CHUNK_JSON) return fail('first chunk is not JSON')
  if (jsonLength % 4) errors.push('JSON chunk length is not 4-byte aligned')
  if (20 + jsonLength > bytes.byteLength) return fail('JSON chunk overruns file')
  let jsonBytes = bytes.subarray(20, 20 + jsonLength)
  const jsonEnd = jsonBytes.lastIndexOf(0x7d) + 1 // '}'
  if (jsonEnd > 0) {
    if (jsonBytes.subarray(jsonEnd).some(byte => byte !== 0x20)) errors.push('JSON chunk padding must be spaces')
    jsonBytes = jsonBytes.subarray(0, jsonEnd)
  }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- parsed glTF JSON
  let json: Record<string, any>
  try {
    json = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(jsonBytes))
  } catch {
    return fail('JSON chunk is not valid UTF-8 JSON')
  }

  let bin: Uint8Array | null = null
  let at = 20 + jsonLength
  if (at < bytes.byteLength) {
    if (at + 8 > bytes.byteLength) return fail('truncated BIN chunk header')
    const binLength = view.getUint32(at, true)
    if (view.getUint32(at + 4, true) !== CHUNK_BIN) errors.push('second chunk is not BIN')
    if (binLength % 4) errors.push('BIN chunk length is not 4-byte aligned')
    if ((at + 8) % 4) errors.push('BIN chunk data is not 4-byte aligned')
    if (at + 8 + binLength > bytes.byteLength) return fail('BIN chunk overruns file')
    bin = bytes.subarray(at + 8, at + 8 + binLength)
    at += 8 + binLength
  }
  if (at !== bytes.byteLength) errors.push('trailing bytes after last chunk')

  if (json.asset?.version !== '2.0') errors.push('asset.version must be "2.0"')
  const buffers = json.buffers ?? []
  if (buffers.length > 1) errors.push('GLB writer emits a single buffer')
  if (buffers[0]) {
    if (buffers[0].uri !== undefined) errors.push('GLB buffer 0 must not have a uri')
    if (!bin) errors.push('buffer 0 without BIN chunk')
    else if (buffers[0].byteLength > bin.byteLength || bin.byteLength - buffers[0].byteLength > 3)
      errors.push('buffer 0 byteLength does not match BIN chunk')
  }
  const bufferViews = json.bufferViews ?? []
  bufferViews.forEach((bv: Record<string, number>, i: number) => {
    const size = buffers[bv.buffer!]?.byteLength
    if (size === undefined) errors.push(`bufferView ${i} references a missing buffer`)
    else if ((bv.byteOffset ?? 0) + bv.byteLength! > size) errors.push(`bufferView ${i} exceeds its buffer`)
  })
  const accessors = json.accessors ?? []
  const accessorData = (index: number) => {
    const accessor = accessors[index]
    const bv = accessor && bufferViews[accessor.bufferView]
    if (!accessor || !bv || !bin) return null
    const offset = (bv.byteOffset ?? 0) + (accessor.byteOffset ?? 0)
    const count = accessor.count * COMPONENTS[accessor.type]!
    if (accessor.componentType === FLOAT)
      return new Float32Array(bin.buffer.slice(bin.byteOffset + offset, bin.byteOffset + offset + count * 4))
    if (accessor.componentType === UNSIGNED_INT)
      return new Uint32Array(bin.buffer.slice(bin.byteOffset + offset, bin.byteOffset + offset + count * 4))
    return null
  }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- glTF JSON is validated field by field
  accessors.forEach((accessor: Record<string, any>, i: number) => {
    const components = COMPONENTS[accessor.type]
    const componentBytes = COMPONENT_BYTES[accessor.componentType]
    const bv = bufferViews[accessor.bufferView]
    if (!components || !componentBytes) return void errors.push(`accessor ${i} has an unknown type`)
    if (!bv) return void errors.push(`accessor ${i} references a missing bufferView`)
    const element = components * componentBytes
    const stride = bv.byteStride ?? element
    const offset = accessor.byteOffset ?? 0
    if (((bv.byteOffset ?? 0) + offset) % componentBytes)
      errors.push(`accessor ${i} is not aligned to its component size`)
    if (accessor.count < 1) errors.push(`accessor ${i} count must be positive`)
    else if (offset + stride * (accessor.count - 1) + element > bv.byteLength)
      errors.push(`accessor ${i} exceeds its bufferView`)
  })
  for (const [m, mesh] of (json.meshes ?? []).entries()) {
    for (const primitive of mesh.primitives ?? []) {
      const position = primitive.attributes?.POSITION
      const positionAccessor = accessors[position]
      if (!positionAccessor) {
        errors.push(`mesh ${m} has no POSITION accessor`)
        continue
      }
      if (!positionAccessor.min || !positionAccessor.max) errors.push(`mesh ${m} POSITION lacks min/max`)
      const data = errors.length ? null : accessorData(position)
      if (data && positionAccessor.min) {
        for (let k = 0; k < 3; k++) {
          let lo = Infinity,
            hi = -Infinity
          for (let v = k; v < data.length; v += 3) {
            lo = Math.min(lo, data[v]!)
            hi = Math.max(hi, data[v]!)
          }
          if (lo !== positionAccessor.min[k] || hi !== positionAccessor.max[k])
            errors.push(`mesh ${m} POSITION min/max mismatch`)
        }
      }
      if (primitive.indices !== undefined) {
        const indices = errors.length ? null : accessorData(primitive.indices)
        if (indices) {
          if (indices.length % 3) errors.push(`mesh ${m} index count is not a multiple of 3`)
          for (const index of indices) {
            if (index >= positionAccessor.count) {
              errors.push(`mesh ${m} index ${index} out of range`)
              break
            }
          }
        }
      }
      if (primitive.material !== undefined && !json.materials?.[primitive.material])
        errors.push(`mesh ${m} references a missing material`)
    }
  }
  return { ok: errors.length === 0, errors, json, bin }
}
