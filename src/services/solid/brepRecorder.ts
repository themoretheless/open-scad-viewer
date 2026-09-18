/**
 * Adapts the OpenSCAD evaluator onto the shared exact-solid builder.
 *
 * The evaluator does not just construct geometry, it also interrogates it (`isEmpty`,
 * `bounds`, `polygons`, `firstVertex3`) while a document is still being evaluated. A
 * purely deferred NURBS graph cannot answer those, so every call is delegated to the
 * real polygon kernel and the exact node is recorded next to it. One evaluation
 * therefore yields both representations: the mesh the Mesh workspace shows and the
 * exact topology the Solid workspace builds.
 *
 * Constructs with no exact analogue are not silently approximated. The handle they
 * produce is marked, the mark propagates through every later operation, and the Solid
 * build refuses the document naming the operation responsible. The polygon track is
 * unaffected, so the same source still opens in Mesh.
 *
 * Every primitive, transform and curve spelling lives in `brepGraph.ts`, which the
 * ModelGraph Text adapter shares.
 */
import type { CadKernelHandle, CadKernelOps } from '../cadKernelOps'
import {
  createBrepGraphBuilder,
  inexact as refuse,
  mirrorMatrix,
  rotationMatrix,
  scaleMatrix,
  translationMatrix,
  type BrepNode,
  type BrepProfile,
  type BrepValue,
} from './brepGraph'

export type { BrepNode } from './brepGraph'

export interface BrepRecording {
  readonly nodes: readonly BrepNode[]
  /** The exact node id for a built handle, or the operation that made it inexact. */
  resolve(handle: CadKernelHandle): { id: string } | { inexact: string }
}

export function createBrepRecordingKernelOps(
  base: CadKernelOps,
): { ops: CadKernelOps; recording: BrepRecording } {
  const graph = createBrepGraphBuilder()
  const records = new WeakMap<CadKernelHandle, BrepValue | BrepProfile>()

  const tag = (handle: CadKernelHandle, value: BrepValue | BrepProfile): CadKernelHandle => {
    records.set(handle, value)
    return handle
  }

  /** The recorded solid for an input, or the reason the caller must give up. */
  const solidOf = (handle: CadKernelHandle): BrepValue => {
    const record = records.get(handle)
    if (record === undefined) return refuse('unrecorded geometry')
    if (record.kind === 'profile') return refuse('2D profile used as a solid')
    return record
  }

  const derive = (
    result: CadKernelHandle,
    inputs: readonly CadKernelHandle[],
    build: (values: BrepValue[]) => BrepValue,
  ): CadKernelHandle => tag(result, build(inputs.map(solidOf)))

  const ops: CadKernelOps = {
    implementationKey: base.implementationKey,

    empty2() { return tag(base.empty2(), refuse('empty 2D geometry')) },
    empty3() { return tag(base.empty3(), refuse('empty geometry')) },

    box(size, center) {
      const [x, y, z] = size
      const min = center ? [-x / 2, -y / 2, -z / 2] : [0, 0, 0]
      const max = center ? [x / 2, y / 2, z / 2] : [x, y, z]
      return tag(base.box(size, center), graph.box(min, max))
    },

    sphere(radius, radialSegments) {
      // The exact sphere carries no tessellation count; segments affect only the mesh track.
      return tag(base.sphere(radius, radialSegments), graph.sphere(radius))
    },

    cylinder(height, radiusBottom, radiusTop, radialSegments, center) {
      const result = base.cylinder(height, radiusBottom, radiusTop, radialSegments, center)
      const solid = graph.cylinder(radiusBottom, radiusTop, height)
      // Both kernels place the base at z = 0; only OpenSCAD's `center` shifts it.
      return tag(result, center
        ? graph.transform(solid, translationMatrix(0, 0, -height / 2))
        : solid)
    },

    rectangle(size, center) {
      const [x, y] = size
      const [x0, y0] = center ? [-x / 2, -y / 2] : [0, 0]
      const ring: [number, number][] = [[x0, y0], [x0 + x, y0], [x0 + x, y0 + y], [x0, y0 + y]]
      return tag(base.rectangle(size, center), { kind: 'profile', loops: [graph.polylineLoop(ring)] })
    },

    circle(radius, radialSegments) {
      return tag(base.circle(radius, radialSegments), {
        kind: 'profile',
        loops: [graph.circleLoop(radius)],
      })
    },

    polygon(rings, fillRule) {
      const result = base.polygon(rings, fillRule)
      if (!rings.length) return tag(result, refuse('empty polygon'))
      return tag(result, { kind: 'profile', loops: rings.map(ring => graph.polylineLoop(ring)) })
    },

    linearExtrude(input, height, slices, twistDegrees, scale, center) {
      const result = base.linearExtrude(input, height, slices, twistDegrees, scale, center)
      const record = records.get(input)
      if (record === undefined || record.kind !== 'profile') {
        return tag(result, refuse('linear_extrude of a non-exact profile'))
      }
      // A twisted or tapered extrusion is no longer a straight prism.
      if (twistDegrees !== 0) return tag(result, refuse('linear_extrude with twist'))
      if (scale[0] !== 1 || scale[1] !== 1) return tag(result, refuse('linear_extrude with scale'))
      return tag(result, graph.extrudeLoops(
        record, center ? -height / 2 : 0, center ? height / 2 : height,
      ))
    },

    rotateExtrude(input, radialSegments, angleDegrees) {
      const result = base.rotateExtrude(input, radialSegments, angleDegrees)
      return tag(result, graph.revolve(solidOf(input), angleDegrees))
    },

    boolean3(operation, inputs) {
      const result = base.boolean3(operation, inputs)
      return derive(result, inputs, values => graph.boolean(operation, values))
    },

    translate(input, offset) {
      return derive(base.translate(input, offset), [input], ([value]) => graph.transform(
        value, translationMatrix(offset[0] ?? 0, offset[1] ?? 0, offset[2] ?? 0),
      ))
    },

    scale(input, factors) {
      const [x, y, z] = [factors[0] ?? 1, factors[1] ?? factors[0] ?? 1, factors[2] ?? factors[0] ?? 1]
      return derive(base.scale(input, factors), [input], ([value]) =>
        graph.transform(value, scaleMatrix(x, y, z)))
    },

    rotate(input, angles) {
      const vector = typeof angles === 'number' ? [0, 0, angles] : angles
      const matrix = rotationMatrix(vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0)
      return derive(base.rotate(input, angles), [input], ([value]) => graph.transform(value, matrix))
    },

    mirror(input, normal) {
      const matrix = mirrorMatrix(normal[0] ?? 0, normal[1] ?? 0, normal[2] ?? 0)
      return derive(base.mirror(input, normal), [input], ([value]) => (matrix === null
        ? refuse('mirror about a degenerate plane')
        : graph.transform(value, matrix)))
    },

    transform3(input, matrix) {
      // The kernel matrix is a column-major 4x4; the graph wants rows.
      const rows = [0, 1, 2].map(row => [0, 1, 2, 3].map(column => matrix[column * 4 + row] ?? 0))
      return derive(base.transform3(input, matrix), [input], ([value]) =>
        graph.transform(value, [...rows, [0, 0, 0, 1]] as [number[], number[], number[], number[]]))
    },

    translateZ(input, distance) {
      return derive(base.translateZ(input, distance), [input], ([value]) =>
        graph.transform(value, translationMatrix(0, 0, distance)))
    },

    mirrorZ(input) {
      return derive(base.mirrorZ(input), [input], ([value]) =>
        graph.transform(value, scaleMatrix(1, 1, -1)))
    },

    asOriginal(input) { return tag(base.asOriginal(input), solidOf(input)) },

    // No exact analogue. The mark travels with the handle and is reported by name
    // if this geometry reaches the Solid output.
    polyhedron(vertices, triangles) { return tag(base.polyhedron(vertices, triangles), refuse('polyhedron')) },
    ofMesh(vertProperties, triVerts) { return tag(base.ofMesh(vertProperties, triVerts), refuse('imported mesh')) },
    hull2(inputs) { return tag(base.hull2(inputs), refuse('hull')) },
    hull3(inputs) { return tag(base.hull3(inputs), refuse('hull')) },
    projection(input, cut) { return tag(base.projection(input, cut), refuse('projection')) },
    offset(input, distance, join, miterLimit, circularSegments) {
      return tag(base.offset(input, distance, join, miterLimit, circularSegments), refuse('offset'))
    },
    minkowskiSum3(left, right) { return tag(base.minkowskiSum3(left, right), refuse('minkowski')) },
    boolean2(operation, inputs) { return tag(base.boolean2(operation, inputs), refuse(`2D ${operation}`)) },
    transform2(input, matrix) { return tag(base.transform2(input, matrix), refuse('2D transform')) },

    // Queries and lifetime: the polygon track answers these unchanged.
    polygons(input) { return base.polygons(input) },
    bounds(input) { return base.bounds(input) },
    firstVertex3(input) { return base.firstVertex3(input) },
    isEmpty(input) { return base.isEmpty(input) },
    originalId(input) { return base.originalId(input) },
    analyzeSolid(input) { return base.analyzeSolid(input) },
    delete(input) { base.delete(input) },
  }

  return {
    ops: Object.freeze(ops),
    recording: {
      nodes: graph.nodes,
      resolve(handle) {
        const value = solidOf(handle)
        return value.kind === 'node' ? { id: value.id } : { inexact: value.operation }
      },
    },
  }
}
