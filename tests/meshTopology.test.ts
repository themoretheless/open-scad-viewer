import { describe, expect, it } from 'vitest'
import { extractSemanticEdges } from '../src/services/meshTopology'

function vertices(positions: number[][]) {
  return Float32Array.from(positions.flatMap(([x, y, z]) => [x, y, z, 0, 0, 1]))
}

describe('semantic mesh edges', () => {
  it('removes the coplanar diagonal from a triangulated quad', () => {
    const result = extractSemanticEdges(
      vertices([
        [0, 0, 0],
        [1, 0, 0],
        [1, 1, 0],
        [0, 1, 0],
      ]),
      Uint32Array.from([0, 1, 2, 0, 2, 3]),
    )

    expect([...result.indices]).toEqual([0, 1, 0, 3, 1, 2, 2, 3])
    expect(result.diagnostics).toEqual({ boundary: 4, crease: 0, nonManifold: 0, degenerate: 0 })
  })

  it('keeps the twelve creases of a triangulated cube but not face diagonals', () => {
    const cubeVertices = vertices([
      [0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0],
      [0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1],
    ])
    const cubeTriangles = Uint32Array.from([
      0, 2, 1, 0, 3, 2,
      4, 5, 6, 4, 6, 7,
      0, 1, 5, 0, 5, 4,
      1, 2, 6, 1, 6, 5,
      2, 3, 7, 2, 7, 6,
      3, 0, 4, 3, 4, 7,
    ])

    const result = extractSemanticEdges(cubeVertices, cubeTriangles, { creaseAngleDegrees: 30 })

    expect(result.indices).toHaveLength(12 * 2)
    expect(result.diagnostics).toEqual({ boundary: 0, crease: 12, nonManifold: 0, degenerate: 0 })
  })

  it('reconstructs topology from duplicated property vertices', () => {
    const duplicated = vertices([
      [0, 0, 0], [1, 0, 0], [1, 1, 0],
      [0, 0, 0], [1, 1, 0], [0, 1, 0],
    ])
    const triangles = Uint32Array.from([0, 1, 2, 3, 4, 5])

    const exactWeld = extractSemanticEdges(duplicated, triangles)
    const manifoldMerge = extractSemanticEdges(duplicated, triangles, {
      mergeFromVert: Uint32Array.from([3, 4]),
      mergeToVert: Uint32Array.from([0, 2]),
      weldCoincidentVertices: false,
    })

    expect([...exactWeld.indices]).toEqual([0, 1, 0, 5, 1, 2, 2, 5])
    expect(manifoldMerge).toEqual(exactWeld)
  })

  it('shows a non-manifold edge once and reports it', () => {
    const result = extractSemanticEdges(
      vertices([
        [0, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
        [0, -1, 0],
      ]),
      Uint32Array.from([
        0, 1, 2,
        1, 0, 3,
        0, 1, 4,
      ]),
    )

    expect(result.indices).toHaveLength(7 * 2)
    expect(result.diagnostics).toEqual({ boundary: 6, crease: 0, nonManifold: 1, degenerate: 0 })
    expect(edgePairs(result.indices)).toContain('0:1')
  })

  it('ignores repeated-index and collinear degenerate triangles', () => {
    const result = extractSemanticEdges(
      vertices([
        [0, 0, 0],
        [1, 0, 0],
        [2, 0, 0],
        [0, 1, 0],
      ]),
      Uint32Array.from([
        0, 0, 1,
        0, 1, 2,
        0, 1, 3,
      ]),
    )

    expect([...result.indices]).toEqual([0, 1, 0, 3, 1, 3])
    expect(result.diagnostics).toEqual({ boundary: 3, crease: 0, nonManifold: 0, degenerate: 2 })
  })

  it('honours the crease threshold and stays deterministic across triangle order', () => {
    const folded = vertices([
      [0, 0, 0],
      [1, 0, 0],
      [0, 1, 0],
      [0, 0, 1],
    ])
    const forward = Uint32Array.from([0, 1, 2, 1, 0, 3])
    const reversed = Uint32Array.from([1, 0, 3, 0, 1, 2])

    const visible = extractSemanticEdges(folded, forward, { creaseAngleDegrees: 45 })
    const hidden = extractSemanticEdges(folded, forward, { creaseAngleDegrees: 100 })
    const reordered = extractSemanticEdges(folded, reversed, { creaseAngleDegrees: 45 })

    expect(visible.diagnostics.crease).toBe(1)
    expect(hidden.diagnostics.crease).toBe(0)
    expect(hidden.indices).toHaveLength(4 * 2)
    expect(reordered).toEqual(visible)
  })

  it('sorts sparse high vertex IDs correctly even when there are only a few valid edges', () => {
    const sparseVertices = new Float32Array(65_537 * 6)
    sparseVertices.set([0, 0, 0], 65_536 * 6)
    sparseVertices.set([1, 0, 0], 256 * 6)
    sparseVertices.set([1, 1, 0], 65_535 * 6)
    sparseVertices.set([0, 1, 0], 255 * 6)
    const result = extractSemanticEdges(sparseVertices, Uint32Array.from([
      255, 255, 65_536, // Skipped occurrence slots precede the valid triangles.
      65_536, 256, 65_535,
      65_536, 65_535, 255,
    ]), { weldCoincidentVertices: false })

    // Four square boundaries in lexicographic vertex-ID order. The diagonal
    // (65535, 65536) is coplanar, independently of the sparse ID assignment.
    expect([...result.indices]).toEqual([255, 65_535, 255, 65_536, 256, 65_535, 256, 65_536])
    expect(result.diagnostics).toEqual({ boundary: 4, crease: 0, nonManifold: 0, degenerate: 1 })
  })

  it.each([10_922, 10_923])('matches the exact strip boundary around the radix-size transition (%i segments)', segments => {
    // Each segment contributes six edge occurrences: these cases straddle
    // 65,536 occurrences while retaining a simple independently known outline.
    const stripVertices = new Float32Array((segments + 1) * 2 * 6)
    const triangles = new Uint32Array(segments * 6)
    const boundary = [[0, 1], [segments * 2, segments * 2 + 1]]
    for (let x = 0; x <= segments; x++) {
      stripVertices.set([x, 0, 0, 0, 0, 1, x, 1, 0, 0, 0, 1], x * 12)
      if (x === segments) continue
      const low = x * 2
      // Reversed segment order forces extraction to establish output order.
      triangles.set([low, low + 2, low + 3, low, low + 3, low + 1], (segments - 1 - x) * 6)
      boundary.push([low, low + 2], [low + 1, low + 3])
    }
    boundary.sort((left, right) => left[0] - right[0] || left[1] - right[1])

    const result = extractSemanticEdges(stripVertices, triangles, { weldCoincidentVertices: false })
    expect(result.indices).toEqual(Uint32Array.from(boundary.flat()))
    expect(result.diagnostics).toEqual({ boundary: segments * 2 + 2, crease: 0, nonManifold: 0, degenerate: 0 })
  })

  it('returns no edges when every triangle is degenerate', () => {
    const result = extractSemanticEdges(vertices([[0, 0, 0], [1, 0, 0], [2, 0, 0]]), Uint32Array.from([0, 0, 1, 0, 1, 2]))
    expect(result.indices).toEqual(new Uint32Array())
    expect(result.diagnostics).toEqual({ boundary: 0, crease: 0, nonManifold: 0, degenerate: 2 })
  })

  it('keeps a large coplanar strip compact and removes every triangulation diagonal', () => {
    const segments = 25_000
    const stripVertices = new Float32Array((segments + 1) * 2 * 6)
    for (let x = 0; x <= segments; x++) {
      for (let row = 0; row < 2; row++) {
        const offset = (x * 2 + row) * 6
        stripVertices[offset] = x
        stripVertices[offset + 1] = row
        stripVertices[offset + 5] = 1
      }
    }
    const triangles = new Uint32Array(segments * 6)
    for (let segment = 0; segment < segments; segment++) {
      const low = segment * 2
      const nextLow = low + 2
      const offset = segment * 6
      triangles.set([low, nextLow, nextLow + 1, low, nextLow + 1, low + 1], offset)
    }

    const result = extractSemanticEdges(stripVertices, triangles, {
      // Empty Manifold merge vectors explicitly select its already-canonical
      // topology and avoid spending memory on an unnecessary position weld.
      mergeFromVert: new Uint32Array(),
      mergeToVert: new Uint32Array(),
    })

    expect(result.indices).toHaveLength((segments * 2 + 2) * 2)
    expect(result.diagnostics).toEqual({
      boundary: segments * 2 + 2,
      crease: 0,
      nonManifold: 0,
      degenerate: 0,
    })
  })

  it('welds coincident positions above the large-mesh threshold without changing topology', () => {
    const vertexCount = 65_536
    const largeVertices = new Float32Array(vertexCount * 6)
    for (let vertex = 0; vertex < vertexCount; vertex++) {
      const offset = vertex * 6
      largeVertices[offset] = vertex + 10
      largeVertices[offset + 1] = vertex % 127
      largeVertices[offset + 5] = 1
    }
    const positions = [
      [0, 0, 0], [1, 0, 0], [1, 1, 0],
      [-0, 0, 0], [1, 1, 0], [0, 1, 0],
    ]
    for (let vertex = 0; vertex < positions.length; vertex++) {
      largeVertices.set(positions[vertex], vertex * 6)
    }

    const result = extractSemanticEdges(
      largeVertices,
      Uint32Array.from([0, 1, 2, 3, 4, 5]),
    )

    expect([...result.indices]).toEqual([0, 1, 0, 5, 1, 2, 2, 5])
    expect(result.diagnostics).toEqual({ boundary: 4, crease: 0, nonManifold: 0, degenerate: 0 })
  })
})

function edgePairs(indices: Uint32Array) {
  const pairs: string[] = []
  for (let i = 0; i < indices.length; i += 2) pairs.push(`${indices[i]}:${indices[i + 1]}`)
  return pairs
}
