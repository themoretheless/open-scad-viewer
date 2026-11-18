import { describe, expect, it } from 'vitest'
import { createSelectionHistory } from '../src/features/directModeler/selectionHistory'
import {
  captureEdges,
  captureFaces,
  edgeSignature,
  resolveEdge,
  resolveEdges,
  resolveFace,
  resolveFaces,
} from '../src/features/directModeler/subshapeSelection'

const face = (normal: number[], center: number[]) => ({ normal, center })
const cubeFaces = [
  face([0, 0, -1], [5, 5, 0]),
  face([0, 0, 1], [5, 5, 10]),
  face([1, 0, 0], [10, 5, 5]),
  face([-1, 0, 0], [0, 5, 5]),
]
const edge = (id: string, a: number[], b: number[]) => ({ id, points: [a, b] })

describe('C15 subshape selection identity', () => {
  it('finds the same face after the face list is renumbered', () => {
    const anchor = captureFaces(cubeFaces, 2, [2, 1])
    const renumbered = [cubeFaces[3], cubeFaces[1], cubeFaces[0], cubeFaces[2]]
    expect(resolveFaces(renumbered, anchor)).toEqual({ faceIndex: 3, openingFaces: [3, 1] })
  })

  it('drops a face whose plane or centre changed instead of choosing another face', () => {
    const anchor = captureFaces(cubeFaces, 1, [1])
    const pushed = cubeFaces.map((f, i) => (i === 1 ? face([0, 0, 1], [5, 5, 12]) : f))
    expect(resolveFaces(pushed, anchor)).toEqual({ faceIndex: -1, openingFaces: [] })
  })

  it('refuses an ambiguous face match', () => {
    const twin = [...cubeFaces, face([0, 0, 1], [5, 5, 10])]
    expect(resolveFace(twin, captureFaces(cubeFaces, 1, []).face!)).toBe(-1)
  })

  it('keeps the remaining openings when the primary face is gone', () => {
    const anchor = captureFaces(cubeFaces, 1, [1, 2])
    expect(resolveFaces([cubeFaces[2], cubeFaces[0]], anchor)).toEqual({ faceIndex: 0, openingFaces: [0] })
  })

  it('resolves edges by persistent topology id first', () => {
    const edges = [edge('e-a', [0, 0, 0], [1, 0, 0]), edge('e-b', [0, 0, 0], [0, 1, 0])]
    const anchor = captureEdges(edges, 1, [1])
    // Same id, moved geometry: the persistent reference wins.
    const moved = [edge('e-b', [0, 0, 2], [0, 1, 2]), edge('e-a', [0, 0, 0], [1, 0, 0])]
    expect(resolveEdges(moved, anchor)).toEqual({ edgeIndex: 0, edgeIndexes: [0] })
  })

  it('resolves index-derived edge ids by their end points in either direction', () => {
    const edges = [edge('B:mesh-edge:0', [0, 0, 0], [1, 0, 0]), edge('B:mesh-edge:1', [1, 0, 0], [1, 1, 0])]
    const signature = edgeSignature(edges[1])
    expect(signature.id).toBe('')
    const reordered = [edge('B:mesh-edge:0', [1, 1, 0], [1, 0, 0]), edge('B:mesh-edge:1', [0, 0, 0], [1, 0, 0])]
    expect(resolveEdge(reordered, signature)).toBe(0)
    expect(resolveEdge([edge('B:edge:0', [0, 0, 0], [1, 0, 0])], signature)).toBe(-1)
  })

  it('remembers the subshape with its body and drops it with the body', () => {
    const memory = createSelectionHistory(),
      state = {}
    const subshape = { body: 'a', ...captureFaces(cubeFaces, 0, [0]), edge: null, edges: [] }
    memory.remember(state, { primary: 'a', extra: ['b'], subshape })
    expect(memory.recall(state, () => true)?.subshape).toEqual(subshape)
    expect(memory.recall(state, id => id !== 'a')).toEqual({ primary: 'b', extra: [], subshape: null })
  })
})
