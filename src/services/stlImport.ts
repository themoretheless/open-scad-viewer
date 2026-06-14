/**
 * Binary STL import to MeshData[].
 * Format: 80-byte header, 4-byte triangle count (uint32 LE),
 * then per triangle: normal (3×f32), v1 (3×f32), v2 (3×f32), v3 (3×f32), 2-byte attribute.
 */
import { identity } from './math3d'
import type { MeshData } from './openscadParser'

/**
 * Parse a binary STL buffer and return a single-element MeshData array
 * containing all triangles.
 */
export function parseSTL(buffer: ArrayBuffer): MeshData[] {
  const view = new DataView(buffer)

  // 80-byte header is skipped
  const triangleCount = view.getUint32(80, true)

  // Validate buffer size: 84 header bytes + 50 bytes per triangle
  const expectedSize = 84 + triangleCount * 50
  if (buffer.byteLength < expectedSize) {
    throw new Error(
      `Invalid STL: expected at least ${expectedSize} bytes, got ${buffer.byteLength}`
    )
  }

  // Each triangle produces 3 unique vertices (STL stores no shared vertices).
  // Each vertex has 6 floats: pos(3) + normal(3).
  const vertexCount = triangleCount * 3
  const vertices = new Float32Array(vertexCount * 6)
  const indices = new Uint32Array(triangleCount * 3)

  let offset = 84

  for (let tri = 0; tri < triangleCount; tri++) {
    // Read face normal
    const nx = view.getFloat32(offset, true);     offset += 4
    const ny = view.getFloat32(offset, true);     offset += 4
    const nz = view.getFloat32(offset, true);     offset += 4

    // Read 3 vertices, storing interleaved pos + normal
    for (let v = 0; v < 3; v++) {
      const vertexIndex = tri * 3 + v
      const base = vertexIndex * 6

      // Position
      vertices[base]     = view.getFloat32(offset, true); offset += 4
      vertices[base + 1] = view.getFloat32(offset, true); offset += 4
      vertices[base + 2] = view.getFloat32(offset, true); offset += 4

      // Normal (same face normal for all 3 vertices of this triangle)
      vertices[base + 3] = nx
      vertices[base + 4] = ny
      vertices[base + 5] = nz

      // Index: sequential since STL has no vertex sharing
      indices[vertexIndex] = vertexIndex
    }

    // Skip 2-byte attribute
    offset += 2
  }

  return [{
    vertices,
    indices,
    color: [0.7, 0.7, 0.7, 1],
    transform: identity(),
  }]
}
