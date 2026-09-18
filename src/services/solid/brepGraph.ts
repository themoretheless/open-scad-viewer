/**
 * The one place that knows how an exact solid is spelled as `modelgraph/nurbs-1` nodes.
 *
 * Both source languages reach the Solid workspace through this builder: the OpenSCAD
 * evaluator via a `CadKernelOps` adapter, and ModelGraph Text by rewriting its compiled
 * document. Keeping the primitive, transform and curve spellings here is what stops the
 * two adapters from drifting apart, since the mapping is identical for both.
 *
 * Nothing here executes geometry. The builder only produces nodes and the rule for
 * refusing a construct that has no exact analogue.
 */

export type BrepNode = Record<string, unknown> & { id: string; op: string }

/** A built value is either an exact node, or the operation that prevented one. */
export type BrepValue = { kind: 'node'; id: string } | { kind: 'inexact'; operation: string }

/** A 2D profile kept as closed loops of curve nodes, ready for `brep_extrude_curves`. */
export type BrepProfile = { kind: 'profile'; loops: string[][] }

export type Matrix4 = [number[], number[], number[], number[]]

const TAU = Math.PI * 2

export interface BrepGraphBuilder {
  readonly nodes: readonly BrepNode[]
  box(min: readonly number[], max: readonly number[]): BrepValue
  sphere(radius: number): BrepValue
  /** A cylinder or, for unequal radii, a frustum. Both sit with the base at z = 0. */
  cylinder(radiusBottom: number, radiusTop: number, height: number): BrepValue
  tube(outerRadius: number, innerRadius: number, height: number): BrepValue
  torus(majorRadius: number, minorRadius: number): BrepValue
  transform(input: BrepValue, matrix: Matrix4): BrepValue
  /** Folds pairwise, because the exact boolean node takes exactly two inputs. */
  boolean(operation: BooleanOperation, inputs: readonly BrepValue[]): BrepValue
  extrudeLoops(profile: BrepProfile, zMin: number, zMax: number): BrepValue
  revolve(input: BrepValue, angleDegrees: number): BrepValue
  /** A polyline loop of degree-1 curves through the given closed ring. */
  polylineLoop(ring: readonly (readonly [number, number])[]): string[]
  /** A closed circle as four exact rational quadratic arcs. */
  circleLoop(radius: number): string[]
}

export type BooleanOperation = 'union' | 'intersection' | 'difference' | 'xor'

export const inexact = (operation: string): BrepValue => ({ kind: 'inexact', operation })

/** Propagates the first refusal among the inputs, otherwise applies `build`. */
export function requireExact(
  inputs: readonly BrepValue[],
  build: (ids: string[]) => BrepValue,
): BrepValue {
  const ids: string[] = []
  for (const input of inputs) {
    if (input.kind === 'inexact') return input
    ids.push(input.id)
  }
  return build(ids)
}

export const translationMatrix = (x: number, y: number, z: number): Matrix4 => [
  [1, 0, 0, x],
  [0, 1, 0, y],
  [0, 0, 1, z],
  [0, 0, 0, 1],
]

export const scaleMatrix = (x: number, y: number, z: number): Matrix4 => [
  [x, 0, 0, 0],
  [0, y, 0, 0],
  [0, 0, z, 0],
  [0, 0, 0, 1],
]

/** Extrinsic X, then Y, then Z rotation, which is the order OpenSCAD applies. */
export function rotationMatrix(degreesX: number, degreesY: number, degreesZ: number): Matrix4 {
  const [rx, ry, rz] = [degreesX, degreesY, degreesZ].map(d => (d * Math.PI) / 180)
  const [cx, sx] = [Math.cos(rx), Math.sin(rx)]
  const [cy, sy] = [Math.cos(ry), Math.sin(ry)]
  const [cz, sz] = [Math.cos(rz), Math.sin(rz)]
  return [
    [cy * cz, cz * sx * sy - cx * sz, cx * cz * sy + sx * sz, 0],
    [cy * sz, cx * cz + sx * sy * sz, -cz * sx + cx * sy * sz, 0],
    [-sy, cy * sx, cx * cy, 0],
    [0, 0, 0, 1],
  ]
}

/** Householder reflection about the plane through the origin with the given normal. */
export function mirrorMatrix(nx: number, ny: number, nz: number): Matrix4 | null {
  const length = Math.hypot(nx, ny, nz)
  if (length === 0) return null
  const [x, y, z] = [nx / length, ny / length, nz / length]
  return [
    [1 - 2 * x * x, -2 * x * y, -2 * x * z, 0],
    [-2 * x * y, 1 - 2 * y * y, -2 * y * z, 0],
    [-2 * x * z, -2 * y * z, 1 - 2 * z * z, 0],
    [0, 0, 0, 1],
  ]
}

export function createBrepGraphBuilder(): BrepGraphBuilder {
  const nodes: BrepNode[] = []
  let counter = 0

  const add = (node: Omit<BrepNode, 'id'>): string => {
    const id = `n${counter++}`
    nodes.push({ ...node, id } as BrepNode)
    return id
  }
  const node = (spec: Omit<BrepNode, 'id'>): BrepValue => ({ kind: 'node', id: add(spec) })

  const curve = (
    degree: number,
    knots: number[],
    controlPoints: number[][],
    weights: number[],
  ): string => add({ op: 'curve', degree, knots, control_points: controlPoints, weights, periodic: false })

  const line = (from: readonly [number, number], to: readonly [number, number]): string =>
    curve(1, [0, 0, 1, 1], [[from[0], from[1], 0], [to[0], to[1], 0]], [1, 1])

  return {
    nodes,

    box: (min, max) => node({ op: 'brep_box', min: [...min], max: [...max] }),
    sphere: radius => node({ op: 'brep_sphere', radius }),
    tube: (outerRadius, innerRadius, height) =>
      node({ op: 'brep_tube', outer_radius: outerRadius, inner_radius: innerRadius, height }),
    torus: (majorRadius, minorRadius) =>
      node({ op: 'brep_torus', major_radius: majorRadius, minor_radius: minorRadius }),

    cylinder: (radiusBottom, radiusTop, height) => (radiusBottom === radiusTop
      ? node({ op: 'brep_cylinder', radius: radiusBottom, height })
      : node({ op: 'brep_frustum', bottom_radius: radiusBottom, top_radius: radiusTop, height })),

    transform: (input, matrix) => requireExact([input], ([id]) =>
      node({ op: 'transform', input: id, matrix: matrix.map(row => [...row]) })),

    boolean: (operation, inputs) => (inputs.length
      ? requireExact(inputs, ids => ids.slice(1).reduce<BrepValue>(
        (left, right) => requireExact([left], ([id]) =>
          node({ op: 'brep_boolean', inputs: [id, right], operation })),
        { kind: 'node', id: ids[0] },
      ))
      : inexact('empty boolean')),

    extrudeLoops: (profile, zMin, zMax) =>
      node({ op: 'brep_extrude_curves', loops: profile.loops.map(loop => [...loop]), z_min: zMin, z_max: zMax }),

    revolve: (input, angleDegrees) => requireExact([input], ([id]) =>
      node({ op: 'brep_revolve', input: id, angle: angleDegrees })),

    polylineLoop: ring => ring.map((from, index) => line(from, ring[(index + 1) % ring.length])),

    circleLoop(radius) {
      const w = Math.SQRT1_2
      // The shoulder of a 90-degree rational quadratic arc sits at radius / cos(45 degrees).
      const shoulder = radius / w
      return [0, 1, 2, 3].map(quadrant => {
        const start = (quadrant * TAU) / 4
        const mid = start + TAU / 8
        const end = start + TAU / 4
        return curve(2, [0, 0, 0, 1, 1, 1], [
          [radius * Math.cos(start), radius * Math.sin(start), 0],
          [shoulder * Math.cos(mid), shoulder * Math.sin(mid), 0],
          [radius * Math.cos(end), radius * Math.sin(end), 0],
        ], [1, w, 1])
      })
    },
  }
}
