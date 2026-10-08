import {callGeometryRust} from '../geometry/kernel'
/**
 * The one place that knows how an exact solid is spelled as `rush/nurbs-1` nodes.
 *
 * Both source languages reach the Solid workspace through this builder: the OpenSCAD
 * evaluator via a `CadKernelOps` adapter, and Rush by rewriting its compiled
 * document. Keeping the primitive, transform and curve spellings here is what stops the
 * two adapters from drifting apart, since the mapping is identical for both.
 *
 * Native constructors supply primitive coordinates and transform matrices. This
 * adapter assigns node IDs and records unsupported constructs for the Solid build.
 */

export type BrepNode = Record<string, unknown> & { id: string; op: string }

/** A built value is either an exact node, or the operation that prevented one. */
export type BrepValue = { kind: 'node'; id: string } | { kind: 'inexact'; operation: string }

/** A 2D profile kept as closed loops of curve nodes, ready for `brep_extrude_curves`. */
export type BrepProfile = { kind: 'profile'; loops: string[][] }

export type Matrix4 = [number[], number[], number[], number[]]


export interface BrepGearNodeSpec {
  module: number
  teeth: number
  height: number
  pressure_angle?: number
  helix_angle?: number
  herringbone?: boolean
  bore?: number
  internal?: boolean
  rim_width?: number
  clearance?: number
  backlash?: number
}

export interface BrepGraphBuilder {
  readonly nodes: readonly BrepNode[]
  box(min: readonly number[], max: readonly number[]): BrepValue
  sphere(radius: number): BrepValue
  /** Involute gear; keys follow the graph node (`pressure_angle`, `helix_angle`, `rim_width`). */
  gear(spec: BrepGearNodeSpec): BrepValue
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
  rectangleLoop(size: readonly [number, number], center: boolean): string[]
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

const nativeMatrix = (operation: string, vector: number[]) =>
  callGeometryRust<Matrix4 | null>('affine_matrix', {operation, vector})

export const translationMatrix = (x: number, y: number, z: number): Matrix4 => nativeMatrix('translate', [x, y, z])!
export const scaleMatrix = (x: number, y: number, z: number): Matrix4 => nativeMatrix('scale', [x, y, z])!

/** Extrinsic X, then Y, then Z rotation, which is the order OpenSCAD applies. */
export const rotationMatrix = (x: number, y: number, z: number): Matrix4 => nativeMatrix('rotate', [x, y, z])!

/** Householder reflection about the plane through the origin with the given normal. */
export const mirrorMatrix = (x: number, y: number, z: number): Matrix4 | null => nativeMatrix('mirror', [x, y, z])

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

  const polylineLoop = (ring: readonly (readonly [number, number])[]): string[] =>
    ring.map((from, index) => line(from, ring[(index + 1) % ring.length]))

  return {
    nodes,

    box: (min, max) => node({ op: 'brep_box', min: [...min], max: [...max] }),
    sphere: radius => node({ op: 'brep_sphere', radius }),
    gear: spec => node({ op: 'brep_gear', ...spec }),
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

    polylineLoop,

    rectangleLoop(size, center) {
      const ring = callGeometryRust<[number, number][]>('planar_rectangle_corners', {size, center})
      return polylineLoop(ring)
    },

    circleLoop(radius) {
      const arcs = callGeometryRust<{degree: number; knots: number[]; controlPoints: number[][]; weights: number[]}[]>('nurbs_circle_quadrants', {radius})
      return arcs.map(arc => curve(arc.degree, arc.knots, arc.controlPoints, arc.weights))
    },
  }
}
