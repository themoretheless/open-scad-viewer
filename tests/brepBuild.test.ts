import { stringifyMeshJson } from '../src/services/meshJson'
import { describe, expect, it } from 'vitest'
import {
  createBrepGraphBuilder,
  rotationMatrix,
  translationMatrix,
  type BrepValue,
} from '../src/services/solid/brepGraph'
import { buildExactSolidBodies, InexactSolidError } from '../src/services/solid/brepBuild'
import { emptyDirectDocument, parseDirectDocument } from '../src/services/directModeling'

const nodeId = (value: BrepValue): string => {
  if (value.kind !== 'node') throw new Error(`Expected an exact node, got ${value.operation}`)
  return value.id
}

/** World bounds of a built body's display mesh. */
function meshBounds(positions: readonly number[]) {
  const min = [Infinity, Infinity, Infinity]
  const max = [-Infinity, -Infinity, -Infinity]
  for (let index = 0; index < positions.length; index++) {
    const axis = index % 3
    min[axis] = Math.min(min[axis], positions[index])
    max[axis] = Math.max(max[axis], positions[index])
  }
  return { min, max }
}

describe('exact solid build', () => {
  it('builds a box as a body carrying both exact topology and a display mesh', () => {
    const graph = createBrepGraphBuilder()
    const box = graph.box([-5, -5, -5], [5, 5, 5])
    const [body] = buildExactSolidBodies(graph.nodes, [{ name: 'Box', id: nodeId(box) }])

    expect(body.brep).toBeDefined()
    expect(body.brep!.faces).toHaveLength(6)
    expect(body.mesh.indices.length).toBeGreaterThan(0)
    const bounds = meshBounds(body.mesh.positions)
    expect(bounds.min).toEqual([-5, -5, -5])
    expect(bounds.max).toEqual([5, 5, 5])
  })

  it('places a translated body where the matrix says, which pins the row convention', () => {
    const graph = createBrepGraphBuilder()
    const moved = graph.transform(graph.box([0, 0, 0], [2, 2, 2]), translationMatrix(10, 0, 0))
    const [body] = buildExactSolidBodies(graph.nodes, [{ name: 'Moved', id: nodeId(moved) }])

    const bounds = meshBounds(body.mesh.positions)
    // A row-major matrix with translation in column 3 moves x by +10 and nothing else.
    expect(bounds.min[0]).toBeCloseTo(10, 9)
    expect(bounds.max[0]).toBeCloseTo(12, 9)
    expect(bounds.min[1]).toBeCloseTo(0, 9)
    expect(bounds.max[2]).toBeCloseTo(2, 9)
  })

  it('rotates about X before Z, which is the order the evaluator applies', () => {
    const graph = createBrepGraphBuilder()
    // A tall box turned 90 degrees about X must lie along -Y.
    const turned = graph.transform(graph.box([0, 0, 0], [1, 1, 10]), rotationMatrix(90, 0, 0))
    const [body] = buildExactSolidBodies(graph.nodes, [{ name: 'Turned', id: nodeId(turned) }])

    const bounds = meshBounds(body.mesh.positions)
    expect(bounds.min[1]).toBeCloseTo(-10, 6)
    expect(bounds.max[1]).toBeCloseTo(0, 6)
    expect(bounds.max[2]).toBeCloseTo(1, 6)
  })

  it('produces a body the Solid document accepts, mesh and B-rep agreeing', () => {
    const graph = createBrepGraphBuilder()
    const solid = graph.cylinder(3, 3, 8)
    const [body] = buildExactSolidBodies(graph.nodes, [{ name: 'Pin', id: nodeId(solid) }])

    // Round-tripping through the document parser runs the body validation, which
    // checks that the display mesh and the exact B-rep describe the same extent.
    const document = { ...emptyDirectDocument(), bodies: [body] }
    expect(() => parseDirectDocument(stringifyMeshJson(document))).not.toThrow()
  })

  it('keeps a group label through document validation, and rejects an empty one', () => {
    const graph = createBrepGraphBuilder()
    const solid = graph.box([0, 0, 0], [1, 1, 1])
    const [body] = buildExactSolidBodies(graph.nodes, [{ name: 'Part', id: nodeId(solid) }])

    const grouped = { ...emptyDirectDocument(), bodies: [{ ...body, group: 'model' }] }
    expect(parseDirectDocument(stringifyMeshJson(grouped)).bodies[0].group).toBe('model')

    const empty = { ...emptyDirectDocument(), bodies: [{ ...body, group: '' }] }
    expect(() => parseDirectDocument(stringifyMeshJson(empty))).toThrow(/group/i)
  })

  it('stores a group source in the document and rejects a duplicate group name', () => {
    const graph = createBrepGraphBuilder()
    const solid = graph.box([0, 0, 0], [1, 1, 1])
    const [body] = buildExactSolidBodies(graph.nodes, [{ name: 'Part', id: nodeId(solid) }])

    const document = {
      ...emptyDirectDocument(),
      bodies: [{ ...body, group: 'rig' }],
      groups: [{ name: 'rig', source: 'cube([1,1,1]);' }],
    }
    const parsed = parseDirectDocument(stringifyMeshJson(document))
    expect(parsed.groups).toEqual([{ name: 'rig', source: 'cube([1,1,1]);' }])

    const duplicated = { ...document, groups: [...document.groups, { name: 'rig', source: '' }] }
    expect(() => parseDirectDocument(stringifyMeshJson(duplicated))).toThrow(/group name/i)
  })

  it('refuses an inexact root by naming the operation responsible', () => {
    const graph = createBrepGraphBuilder()
    expect(() => buildExactSolidBodies(graph.nodes, [{ name: 'Blob', inexact: 'hull' }]))
      .toThrow(InexactSolidError)
    expect(() => buildExactSolidBodies(graph.nodes, [{ name: 'Blob', inexact: 'hull' }]))
      .toThrow(/hull/)
  })
})
