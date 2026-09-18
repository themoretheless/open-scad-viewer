/**
 * Records an exact `modelgraph/nurbs-1` graph alongside an ordinary polygon build.
 *
 * The OpenSCAD evaluator does not just construct geometry, it also interrogates it
 * (`isEmpty`, `bounds`, `polygons`, `firstVertex3`) while a document is still being
 * evaluated. A purely deferred B-rep graph cannot answer those, so every call is
 * delegated to the real polygon kernel and the exact node is recorded next to it.
 * One evaluation therefore yields both representations: the mesh the Mesh workspace
 * shows and the exact topology the Solid workspace builds.
 *
 * Constructs with no exact analogue (`hull`, `projection`, `offset`, `minkowski`,
 * `polyhedron`, 2D booleans) are not silently approximated. The handle they produce
 * is marked, the mark propagates through every later operation, and the Solid build
 * refuses the document naming the operation responsible. The polygon track is
 * unaffected, so the same source still opens in Mesh.
 */
import type { CadKernelHandle, CadKernelOps } from '../cadKernelOps'

export type BrepNode = Record<string, unknown> & { id: string; op: string }

/** A handle either maps to an exact node, or carries the reason it cannot. */
type Record_ = { kind: 'node'; id: string } | { kind: 'inexact'; operation: string }

/** A recorded 2D profile, kept as closed loops of curve nodes for `brep_extrude_curves`. */
type Profile = { kind: 'profile'; loops: string[][] }

export interface BrepRecording {
  readonly nodes: readonly BrepNode[]
  /** The exact node id for a built handle, or the operation that made it inexact. */
  resolve(handle: CadKernelHandle): { id: string } | { inexact: string }
}

const TAU = Math.PI * 2

export function createBrepRecordingKernelOps(
  base: CadKernelOps,
): { ops: CadKernelOps; recording: BrepRecording } {
  const nodes: BrepNode[] = []
  const records = new WeakMap<CadKernelHandle, Record_ | Profile>()
  let counter = 0

  const add = (node: Omit<BrepNode, 'id'>): string => {
    const id = `n${counter++}`
    nodes.push({ ...node, id } as BrepNode)
    return id
  }

  /** Tags a handle with an exact node id. */
  const exact = (handle: CadKernelHandle, id: string): CadKernelHandle => {
    records.set(handle, { kind: 'node', id })
    return handle
  }

  /** Tags a handle as having no exact analogue. */
  const inexact = (handle: CadKernelHandle, operation: string): CadKernelHandle => {
    records.set(handle, { kind: 'inexact', operation })
    return handle
  }

  const profile = (handle: CadKernelHandle, loops: string[][]): CadKernelHandle => {
    records.set(handle, { kind: 'profile', loops })
    return handle
  }

  /** The recorded solid node for an input, or the reason the caller must give up. */
  const solidOf = (handle: CadKernelHandle): { id: string } | { inexact: string } => {
    const record = records.get(handle)
    if (record === undefined) return { inexact: 'unrecorded geometry' }
    if (record.kind === 'node') return { id: record.id }
    if (record.kind === 'profile') return { inexact: '2D profile used as a solid' }
    return { inexact: record.operation }
  }

  /** Applies `build` when every input is exact, otherwise propagates the first reason. */
  const derive = (
    result: CadKernelHandle,
    inputs: readonly CadKernelHandle[],
    build: (ids: string[]) => string,
  ): CadKernelHandle => {
    const ids: string[] = []
    for (const input of inputs) {
      const resolved = solidOf(input)
      if ('inexact' in resolved) return inexact(result, resolved.inexact)
      ids.push(resolved.id)
    }
    return exact(result, build(ids))
  }

  const transformNode = (input: string, matrix: number[][]): string =>
    add({ op: 'transform', input, matrix })

  const translation = (x: number, y: number, z: number): number[][] => [
    [1, 0, 0, x],
    [0, 1, 0, y],
    [0, 0, 1, z],
    [0, 0, 0, 1],
  ]

  /** A degree-1 curve node through the given points. */
  const line = (points: readonly (readonly [number, number])[]): string => add({
    op: 'curve',
    degree: 1,
    knots: points.map((_, index) => index / (points.length - 1))
      .flatMap((value, index) => (index === 0 || index === points.length - 1 ? [value, value] : [value])),
    control_points: points.map(([x, y]) => [x, y, 0]),
    weights: points.map(() => 1),
    periodic: false,
  })

  /**
   * A closed rational quadratic circle: four 90-degree arcs sharing one knot vector.
   * The corner weights are cos(45 degrees), which is what makes the arcs exact.
   */
  const circleLoop = (radius: number): string[] => {
    const w = Math.SQRT1_2
    const arcs: string[] = []
    for (let quadrant = 0; quadrant < 4; quadrant++) {
      const start = (quadrant * TAU) / 4
      const mid = start + TAU / 8
      const end = start + TAU / 4
      // The shoulder point of a 90-degree arc sits at radius / cos(45 degrees).
      const shoulder = radius / w
      arcs.push(add({
        op: 'curve',
        degree: 2,
        knots: [0, 0, 0, 1, 1, 1],
        control_points: [
          [radius * Math.cos(start), radius * Math.sin(start), 0],
          [shoulder * Math.cos(mid), shoulder * Math.sin(mid), 0],
          [radius * Math.cos(end), radius * Math.sin(end), 0],
        ],
        weights: [1, w, 1],
        periodic: false,
      }))
    }
    return arcs
  }

  const ops: CadKernelOps = {
    implementationKey: base.implementationKey,

    empty2() { return inexact(base.empty2(), 'empty 2D geometry') },
    empty3() { return inexact(base.empty3(), 'empty geometry') },

    box(size, center) {
      const [x, y, z] = size
      const min = center ? [-x / 2, -y / 2, -z / 2] : [0, 0, 0]
      const max = center ? [x / 2, y / 2, z / 2] : [x, y, z]
      return exact(base.box(size, center), add({ op: 'brep_box', min, max }))
    },

    sphere(radius, radialSegments) {
      // The exact sphere carries no tessellation count; segments affect only the mesh track.
      return exact(base.sphere(radius, radialSegments), add({ op: 'brep_sphere', radius }))
    },

    cylinder(height, radiusBottom, radiusTop, radialSegments, center) {
      const result = base.cylinder(height, radiusBottom, radiusTop, radialSegments, center)
      const solid = radiusBottom === radiusTop
        ? add({ op: 'brep_cylinder', radius: radiusBottom, height })
        : add({ op: 'brep_frustum', bottom_radius: radiusBottom, top_radius: radiusTop, height })
      // Both kernels place the base at z = 0; only OpenSCAD's `center` shifts it.
      return exact(result, center ? transformNode(solid, translation(0, 0, -height / 2)) : solid)
    },

    rectangle(size, center) {
      const [x, y] = size
      const [x0, y0] = center ? [-x / 2, -y / 2] : [0, 0]
      const [x1, y1] = [x0 + x, y0 + y]
      const corners: [number, number][] = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
      const loop = corners.map((from, index) => line([from, corners[(index + 1) % corners.length]]))
      return profile(base.rectangle(size, center), [loop])
    },

    circle(radius, radialSegments) {
      return profile(base.circle(radius, radialSegments), [circleLoop(radius)])
    },

    polygon(rings, fillRule) {
      const result = base.polygon(rings, fillRule)
      if (!rings.length) return inexact(result, 'empty polygon')
      const loops = rings.map(ring => ring.map(
        (from, index) => line([from, ring[(index + 1) % ring.length]]),
      ))
      return profile(result, loops)
    },

    linearExtrude(input, height, slices, twistDegrees, scale, center) {
      const result = base.linearExtrude(input, height, slices, twistDegrees, scale, center)
      const record = records.get(input)
      if (record === undefined || record.kind !== 'profile') {
        return inexact(result, 'linear_extrude of a non-exact profile')
      }
      // A twisted or tapered extrusion is no longer a straight prism.
      if (twistDegrees !== 0) return inexact(result, 'linear_extrude with twist')
      if (scale[0] !== 1 || scale[1] !== 1) return inexact(result, 'linear_extrude with scale')
      return exact(result, add({
        op: 'brep_extrude_curves',
        loops: record.loops,
        z_min: center ? -height / 2 : 0,
        z_max: center ? height / 2 : height,
      }))
    },

    rotateExtrude(input, radialSegments, angleDegrees) {
      const result = base.rotateExtrude(input, radialSegments, angleDegrees)
      const resolved = solidOf(input)
      if ('inexact' in resolved) return inexact(result, resolved.inexact)
      return exact(result, add({ op: 'brep_revolve', input: resolved.id, angle: angleDegrees }))
    },

    boolean3(operation, inputs) {
      const result = base.boolean3(operation, inputs)
      if (!inputs.length) return inexact(result, 'empty boolean')
      return derive(result, inputs, ids => ids.reduce(
        (left, right) => add({ op: 'brep_boolean', inputs: [left, right], operation }),
      ))
    },

    translate(input, offset) {
      const result = base.translate(input, offset)
      return derive(result, [input], ([id]) => transformNode(
        id, translation(offset[0] ?? 0, offset[1] ?? 0, offset[2] ?? 0),
      ))
    },

    scale(input, factors) {
      const result = base.scale(input, factors)
      const [x, y, z] = [factors[0] ?? 1, factors[1] ?? factors[0] ?? 1, factors[2] ?? factors[0] ?? 1]
      return derive(result, [input], ([id]) => transformNode(id, [
        [x, 0, 0, 0], [0, y, 0, 0], [0, 0, z, 0], [0, 0, 0, 1],
      ]))
    },

    rotate(input, angles) {
      const result = base.rotate(input, angles)
      const vector = typeof angles === 'number' ? [0, 0, angles] : angles
      const [rx, ry, rz] = [vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0].map(d => (d * Math.PI) / 180)
      const [cx, sx, cy, sy, cz, sz] = [
        Math.cos(rx), Math.sin(rx), Math.cos(ry), Math.sin(ry), Math.cos(rz), Math.sin(rz),
      ]
      // OpenSCAD applies X, then Y, then Z.
      const matrix = [
        [cy * cz, cz * sx * sy - cx * sz, cx * cz * sy + sx * sz, 0],
        [cy * sz, cx * cz + sx * sy * sz, -cz * sx + cx * sy * sz, 0],
        [-sy, cy * sx, cx * cy, 0],
        [0, 0, 0, 1],
      ]
      return derive(result, [input], ([id]) => transformNode(id, matrix))
    },

    mirror(input, normal) {
      const result = base.mirror(input, normal)
      const [nx, ny, nz] = [normal[0] ?? 0, normal[1] ?? 0, normal[2] ?? 0]
      const length = Math.hypot(nx, ny, nz)
      if (length === 0) return inexact(result, 'mirror about a degenerate plane')
      const [x, y, z] = [nx / length, ny / length, nz / length]
      const matrix = [
        [1 - 2 * x * x, -2 * x * y, -2 * x * z, 0],
        [-2 * x * y, 1 - 2 * y * y, -2 * y * z, 0],
        [-2 * x * z, -2 * y * z, 1 - 2 * z * z, 0],
        [0, 0, 0, 1],
      ]
      return derive(result, [input], ([id]) => transformNode(id, matrix))
    },

    transform3(input, matrix) {
      const result = base.transform3(input, matrix)
      // The kernel matrix is a column-major 4x4; the graph wants rows.
      const rows = [0, 1, 2].map(row => [0, 1, 2, 3].map(column => matrix[column * 4 + row] ?? 0))
      return derive(result, [input], ([id]) => transformNode(id, [...rows, [0, 0, 0, 1]]))
    },

    translateZ(input, distance) {
      const result = base.translateZ(input, distance)
      return derive(result, [input], ([id]) => transformNode(id, translation(0, 0, distance)))
    },

    mirrorZ(input) {
      const result = base.mirrorZ(input)
      return derive(result, [input], ([id]) => transformNode(id, [
        [1, 0, 0, 0], [0, 1, 0, 0], [0, 0, -1, 0], [0, 0, 0, 1],
      ]))
    },

    asOriginal(input) {
      const result = base.asOriginal(input)
      const resolved = solidOf(input)
      return 'inexact' in resolved ? inexact(result, resolved.inexact) : exact(result, resolved.id)
    },

    // No exact analogue. The mark travels with the handle and is reported by name
    // if this geometry reaches the Solid output.
    polyhedron(vertices, triangles) { return inexact(base.polyhedron(vertices, triangles), 'polyhedron') },
    ofMesh(vertProperties, triVerts) { return inexact(base.ofMesh(vertProperties, triVerts), 'imported mesh') },
    hull2(inputs) { return inexact(base.hull2(inputs), 'hull') },
    hull3(inputs) { return inexact(base.hull3(inputs), 'hull') },
    projection(input, cut) { return inexact(base.projection(input, cut), 'projection') },
    offset(input, distance, join, miterLimit, circularSegments) {
      return inexact(base.offset(input, distance, join, miterLimit, circularSegments), 'offset')
    },
    minkowskiSum3(left, right) { return inexact(base.minkowskiSum3(left, right), 'minkowski') },
    boolean2(operation, inputs) { return inexact(base.boolean2(operation, inputs), `2D ${operation}`) },
    transform2(input, matrix) { return inexact(base.transform2(input, matrix), '2D transform') },

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
    recording: { nodes, resolve: solidOf },
  }
}
