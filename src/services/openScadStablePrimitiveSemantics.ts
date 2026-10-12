export { resolveOpenScadCube, resolveOpenScadSquare, resolveOpenScadSphere, resolveOpenScadCircle, resolveOpenScadCylinder, resolveOpenScadPolyhedron } from './openScadStablePrimitiveSemantics/resolveOpenScadCube'
export { resolveOpenScadPolygon } from './openScadStablePrimitiveSemantics/resolveOpenScadPolygon'
/**
 * Kernel-neutral parameter plans for the OpenSCAD 2021.01 primitives.
 *
 * These resolvers stop at the language/kernel boundary: they preserve the
 * permissive conversions and empty-object decisions, but never construct a
 * Manifold or parser node.  Host limits are opt-in and are always surfaced as
 * an explicit reduction instead of silently truncating topology.
 */

export type OpenScadStablePrimitiveName =
  | 'cube'
  | 'square'
  | 'sphere'
  | 'circle'
  | 'cylinder'
  | 'polyhedron'
  | 'polygon'

export type OpenScadStablePrimitiveWarningCode =
  | 'OPENSCAD_PRIMITIVE_SIZE_DEFAULTED'
  | 'OPENSCAD_PRIMITIVE_RANGE_EMPTY'
  | 'OPENSCAD_PRIMITIVE_RADIUS_SHADOWED'
  | 'OPENSCAD_CYLINDER_AMBIGUOUS_RADII'
  | 'OPENSCAD_POLYHEDRON_TRIANGLES_DEPRECATED'
  | 'OPENSCAD_PRIMITIVE_POINT_INVALID'
  | 'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS'
  | 'OPENSCAD_PRIMITIVE_SAFETY_REDUCTION'

export interface OpenScadStablePrimitiveWarning {
  readonly code: OpenScadStablePrimitiveWarningCode
  readonly message: string
  readonly primitive: OpenScadStablePrimitiveName
  readonly field?: string
  readonly value?: unknown
  readonly index?: number
  readonly limit?: number
  readonly actual?: number
}

export interface OpenScadStablePrimitiveContext {
  readonly warn: (warning: OpenScadStablePrimitiveWarning) => void
  /** Mirrors the optional OpenSCAD parameter-range diagnostic switch. */
  readonly checkParameterRanges?: boolean
}

export const SILENT_CONTEXT: OpenScadStablePrimitiveContext = Object.freeze({ warn() {} })

export type OpenScadVec2 = readonly [number, number]
export type OpenScadVec3 = readonly [number, number, number]

export interface OpenScadPrimitiveSafetyLimits {
  readonly maximumPoints?: number
  readonly maximumFacesOrPaths?: number
  readonly maximumIndices?: number
}

export interface OpenScadPrimitiveReduction {
  readonly reason: 'input-limit'
  readonly resource: 'points' | 'faces' | 'paths' | 'indices'
  readonly actual: number
  readonly limit: number
}

interface OpenScadPrimitivePlanBase {
  readonly kind: OpenScadStablePrimitiveName
  readonly empty: boolean
  readonly reduced: boolean
  readonly reduction: OpenScadPrimitiveReduction | null
}

export interface OpenScadBoxPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'cube' | 'square'
  readonly dimensions: OpenScadVec3 | OpenScadVec2
  readonly center: boolean
  readonly sizeSource: 'default' | 'scalar' | 'vector' | 'partial-vector'
}

export interface OpenScadRadialPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'sphere' | 'circle'
  readonly radius: number
  readonly radiusSource: 'default' | 'radius' | 'diameter'
  /** Radius passed to the shared fragment resolver. */
  readonly fragmentRadius: number
}

export interface OpenScadCylinderPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'cylinder'
  readonly height: number
  readonly radius1: number
  readonly radius2: number
  readonly center: boolean
  readonly radius1Source: 'default' | 'radius' | 'diameter' | 'radius1' | 'diameter1'
  readonly radius2Source: 'default' | 'radius' | 'diameter' | 'radius2' | 'diameter2'
  /** Largest end radius passed to the shared fragment resolver. */
  readonly fragmentRadius: number
}

export interface OpenScadPolyhedronPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'polyhedron'
  /** Coordinate polygons in authored face order, after index conversion/skips. */
  readonly polygons: readonly (readonly OpenScadVec3[])[]
  readonly facesSource: 'faces' | 'triangles' | 'default'
  readonly convexity: number
  /** A referenced malformed point ends conversion at that exact face. */
  readonly aborted: boolean
  readonly sourcePointCount: number
}

export interface OpenScadPolygonPrimitivePlan extends OpenScadPrimitivePlanBase {
  readonly kind: 'polygon'
  readonly points: readonly OpenScadVec2[]
  readonly outlines: readonly (readonly OpenScadVec2[])[]
  readonly pathSource: 'implicit' | 'explicit'
  readonly convexity: number
  readonly aborted: boolean
}

export interface OpenScadBoxPrimitiveInput {
  readonly size?: unknown
  readonly center?: unknown
}

export interface OpenScadRadialPrimitiveInput {
  readonly r?: unknown
  readonly d?: unknown
}

export interface OpenScadCylinderPrimitiveInput extends OpenScadRadialPrimitiveInput {
  readonly h?: unknown
  readonly r1?: unknown
  readonly r2?: unknown
  readonly d1?: unknown
  readonly d2?: unknown
  readonly center?: unknown
}

export interface OpenScadPolyhedronPrimitiveInput {
  readonly points?: unknown
  readonly faces?: unknown
  readonly triangles?: unknown
  readonly convexity?: unknown
  readonly limits?: OpenScadPrimitiveSafetyLimits
}

export interface OpenScadPolygonPrimitiveInput {
  readonly points?: unknown
  readonly paths?: unknown
  readonly convexity?: unknown
  readonly limits?: OpenScadPrimitiveSafetyLimits
}

export interface RadiusPair {
  readonly value: number
  readonly source: 'default' | 'radius' | 'diameter'
  readonly supplied: boolean
}

export interface NormalizedLimits {
  readonly maximumPoints: number
  readonly maximumFacesOrPaths: number
  readonly maximumIndices: number
}
