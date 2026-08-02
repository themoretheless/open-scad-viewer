import type { MeshData } from '../core/mesh'
import { transformPoint, type Mat4, type Vec3 } from './math3d'

export const MAX_EXPORT_TRIANGLES = 750_000

export class MeshExportError extends Error {
  constructor(readonly code: 'invalid-mesh' | 'too-many-triangles', message: string) {
    super(message)
    this.name = 'MeshExportError'
  }
}

function validateMeshes(meshes: readonly MeshData[]) {
  let triangles = 0
  for (const mesh of meshes) {
    if (mesh.vertices.length % 6 !== 0 || mesh.indices.length % 3 !== 0
      || mesh.transform.length !== 16 || !mesh.transform.every(Number.isFinite)
      || mesh.transform[12] !== 0 || mesh.transform[13] !== 0
      || mesh.transform[14] !== 0 || mesh.transform[15] !== 1
      || !mesh.vertices.every(Number.isFinite)) {
      throw new MeshExportError('invalid-mesh', 'Mesh export requires finite affine triangle data')
    }
    const vertexCount = mesh.vertices.length / 6
    if (mesh.indices.some(index => index >= vertexCount)) {
      throw new MeshExportError('invalid-mesh', 'Mesh export contains an out-of-range vertex index')
    }
    triangles += mesh.indices.length / 3
    if (triangles > MAX_EXPORT_TRIANGLES) {
      throw new MeshExportError('too-many-triangles', `Export exceeds ${MAX_EXPORT_TRIANGLES} triangles`)
    }
  }
}

function vertex(mesh: MeshData, index: number, float32 = false): Vec3 | null {
  const offset = index * 6
  if (!Number.isSafeInteger(index) || offset < 0 || offset + 2 >= mesh.vertices.length) return null
  const point = transformPoint(mesh.transform, [mesh.vertices[offset], mesh.vertices[offset + 1], mesh.vertices[offset + 2]])
  const encoded = float32 ? point.map(Math.fround) as Vec3 : point
  return encoded.every(Number.isFinite) ? encoded : null
}

function determinant3(transform: Mat4): number {
  const [a, b, c, , d, e, f, , g, h, i] = transform
  return a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

function normal(a: Vec3, b: Vec3, c: Vec3): Vec3 | null {
  const ab: Vec3 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
  const ac: Vec3 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]]
  const cross: Vec3 = [
    ab[1] * ac[2] - ab[2] * ac[1],
    ab[2] * ac[0] - ab[0] * ac[2],
    ab[0] * ac[1] - ab[1] * ac[0],
  ]
  const length = Math.hypot(...cross)
  return length > 0 && Number.isFinite(length)
    ? [cross[0] / length, cross[1] / length, cross[2] / length]
    : null
}

interface Triangle { a: Vec3; b: Vec3; c: Vec3; normal: Vec3 }

function triangleAt(mesh: MeshData, offset: number, float32: boolean): Triangle | null {
  const mirrored = determinant3(mesh.transform) < 0
  const a = vertex(mesh, mesh.indices[offset], float32)
  const second = vertex(mesh, mesh.indices[offset + (mirrored ? 2 : 1)], float32)
  const third = vertex(mesh, mesh.indices[offset + (mirrored ? 1 : 2)], float32)
  if (!a || !second || !third) return null
  const faceNormal = normal(a, second, third)
  return faceNormal ? { a, b: second, c: third, normal: faceNormal } : null
}

function visitTriangles(meshes: readonly MeshData[], float32: boolean, visit: (triangle: Triangle) => void): number {
  let count = 0
  for (const mesh of meshes) {
    for (let offset = 0; offset + 2 < mesh.indices.length; offset += 3) {
      const triangle = triangleAt(mesh, offset, float32)
      if (!triangle) continue
      count++
      visit(triangle)
    }
  }
  return count
}

/** Build a standard little-endian binary STL with transforms baked in. */
export function buildBinaryStl(meshes: readonly MeshData[], name = 'OpenSCAD Viewer'): Uint8Array {
  validateMeshes(meshes)
  const triangleCount = visitTriangles(meshes, true, () => undefined)
  const output = new Uint8Array(84 + triangleCount * 50)
  const header = encodeStlHeader(name)
  output.set(header, 0)
  const view = new DataView(output.buffer)
  view.setUint32(80, triangleCount, true)
  let byte = 84
  visitTriangles(meshes, true, triangle => {
    for (const point of [triangle.normal, triangle.a, triangle.b, triangle.c]) {
      view.setFloat32(byte, point[0], true)
      view.setFloat32(byte + 4, point[1], true)
      view.setFloat32(byte + 8, point[2], true)
      byte += 12
    }
    view.setUint16(byte, 0, true)
    byte += 2
  })
  return output
}

/** Build a portable OBJ with transforms baked in and invalid triangles omitted. */
export function buildObj(meshes: readonly MeshData[]): string {
  validateMeshes(meshes)
  const lines = ['# Exported by OpenSCAD Viewer']
  let emittedTriangles = 0
  let nextObjVertex = 1
  meshes.forEach((mesh, meshIndex) => {
    lines.push(`o result_${meshIndex + 1}`)
    const referenced = new Uint8Array(mesh.vertices.length / 6)
    for (let offset = 0; offset < mesh.indices.length; offset += 3) {
      if (!triangleAt(mesh, offset, false)) continue
      referenced[mesh.indices[offset]] = 1
      referenced[mesh.indices[offset + 1]] = 1
      referenced[mesh.indices[offset + 2]] = 1
    }
    const mapped = new Int32Array(referenced.length)
    mapped.fill(-1)
    for (let index = 0; index < mesh.vertices.length / 6; index++) {
      if (!referenced[index]) continue
      const point = vertex(mesh, index)
      if (!point) continue
      mapped[index] = nextObjVertex++
      lines.push(`v ${point[0]} ${point[1]} ${point[2]}`)
    }
    const mirrored = determinant3(mesh.transform) < 0
    for (let offset = 0; offset + 2 < mesh.indices.length; offset += 3) {
      const raw = [mesh.indices[offset], mesh.indices[offset + 1], mesh.indices[offset + 2]]
      if (mirrored) [raw[1], raw[2]] = [raw[2], raw[1]]
      const mappedIndices = raw.map(index => mapped[index])
      if (mappedIndices.some(index => index < 1)) continue
      const a = vertex(mesh, raw[0]), b = vertex(mesh, raw[1]), c = vertex(mesh, raw[2])
      if (!a || !b || !c || !normal(a, b, c)) continue
      emittedTriangles++
      lines.push(`f ${mappedIndices[0]} ${mappedIndices[1]} ${mappedIndices[2]}`)
    }
  })
  return `${lines.join('\n')}\n`
}

function encodeStlHeader(name: string): Uint8Array {
  const encoder = new TextEncoder()
  const output = new Uint8Array(80)
  let offset = 0
  for (const character of `OpenSCAD Viewer binary: ${name}`) {
    const bytes = encoder.encode(character)
    if (offset + bytes.length > output.length) break
    output.set(bytes, offset)
    offset += bytes.length
  }
  return output
}
