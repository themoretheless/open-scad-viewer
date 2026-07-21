import type { MeshData } from '../core/mesh'
import { transformPoint, type Vec3 } from './math3d'

function vertex(mesh: MeshData, index: number): Vec3 | null {
  const offset = index * 6
  if (offset < 0 || offset + 2 >= mesh.vertices.length) return null
  const point = transformPoint(mesh.transform, [mesh.vertices[offset], mesh.vertices[offset + 1], mesh.vertices[offset + 2]])
  return point.every(Number.isFinite) ? point : null
}

function normal(a: Vec3, b: Vec3, c: Vec3): Vec3 {
  const ab: Vec3 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
  const ac: Vec3 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]]
  const cross: Vec3 = [
    ab[1] * ac[2] - ab[2] * ac[1],
    ab[2] * ac[0] - ab[0] * ac[2],
    ab[0] * ac[1] - ab[1] * ac[0],
  ]
  const length = Math.hypot(...cross)
  return length > 1e-20 ? [cross[0] / length, cross[1] / length, cross[2] / length] : [0, 0, 0]
}

interface Triangle { a: Vec3; b: Vec3; c: Vec3 }

function triangles(meshes: MeshData[]): Triangle[] {
  const result: Triangle[] = []
  for (const mesh of meshes) {
    for (let offset = 0; offset + 2 < mesh.indices.length; offset += 3) {
      const a = vertex(mesh, mesh.indices[offset])
      const b = vertex(mesh, mesh.indices[offset + 1])
      const c = vertex(mesh, mesh.indices[offset + 2])
      if (a && b && c) result.push({ a, b, c })
    }
  }
  return result
}

/** Build a standard little-endian binary STL with transforms baked in. */
export function buildBinaryStl(meshes: MeshData[], name = 'OpenSCAD Viewer'): Uint8Array {
  const all = triangles(meshes)
  const output = new Uint8Array(84 + all.length * 50)
  // Truncate by BYTES after encoding: 80 multibyte characters encode to up
  // to 240 UTF-8 bytes, which overflowed the 80-byte header slot (and threw
  // a RangeError for small scenes).
  const header = new TextEncoder().encode(name).subarray(0, 80)
  output.set(header, 0)
  const view = new DataView(output.buffer)
  view.setUint32(80, all.length, true)
  let byte = 84
  for (const triangle of all) {
    const n = normal(triangle.a, triangle.b, triangle.c)
    for (const point of [n, triangle.a, triangle.b, triangle.c]) {
      view.setFloat32(byte, point[0], true)
      view.setFloat32(byte + 4, point[1], true)
      view.setFloat32(byte + 8, point[2], true)
      byte += 12
    }
    view.setUint16(byte, 0, true)
    byte += 2
  }
  return output
}

/** Build a portable OBJ with one object and material color per result mesh. */
export function buildObj(meshes: MeshData[]): string {
  const lines = ['# Exported by OpenSCAD Viewer']
  let vertexBase = 1
  meshes.forEach((mesh, meshIndex) => {
    lines.push(`o result_${meshIndex + 1}`)
    for (let index = 0; index < mesh.vertices.length / 6; index++) {
      const point = vertex(mesh, index)
      if (!point) lines.push('v 0 0 0')
      else lines.push(`v ${point[0]} ${point[1]} ${point[2]}`)
    }
    for (let offset = 0; offset + 2 < mesh.indices.length; offset += 3) {
      lines.push(`f ${vertexBase + mesh.indices[offset]} ${vertexBase + mesh.indices[offset + 1]} ${vertexBase + mesh.indices[offset + 2]}`)
    }
    vertexBase += mesh.vertices.length / 6
  })
  return `${lines.join('\n')}\n`
}
