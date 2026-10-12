/**
 * Kernel-neutral OpenSCAD 2021.01 geometry-module rules.
 *
 * This file intentionally has no parser or Manifold dependency.  It resolves
 * language parameters and describes the small amount of kernel adaptation
 * needed by the independent executor (notably OpenSCAD slices versus
 * Manifold's interior-division count).
 */

import type { RangeValue } from './openscadCompiler'
import {
  OPENSCAD_2021_GEOMETRY_EPSILON,
  OPENSCAD_DEFAULT_FA,
  OPENSCAD_DEFAULT_FN,
  OPENSCAD_DEFAULT_FS,
  OPENSCAD_MIN_FA,
  OPENSCAD_MIN_FS,
  resolveOpenScadSweepFragments,
  type OpenScadStableModuleSemanticsContext,
} from './openScadStableModuleSemantics'
export { resolveOpenScadLinearExtrude, resolveOpenScad2dMultmatrix, openScadMinkowski2dViaProduct, resolveOpenScadChildrenSelection } from './openScadStableGeometrySemantics/resolveOpenScadLinearExtrude'
export { resolveOpenScadResize, resolveOpenScadRotateExtrude } from './openScadStableGeometrySemantics/resolveOpenScadResize'

export type OpenScadPoint2 = readonly [number, number]
export type OpenScadScale2 = readonly [number, number]

export type OpenScadStableGeometryWarningCode =
  | 'OPENSCAD_LINEAR_EXTRUDE_HEIGHT_DEFAULTED'
  | 'OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED'
  | 'OPENSCAD_LINEAR_EXTRUDE_SLICES_CLAMPED'
  | 'OPENSCAD_CHILDREN_BAD_SELECTION'
  | 'OPENSCAD_CHILDREN_BAD_INDEX'
  | 'OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS'
  | 'OPENSCAD_CHILDREN_RANGE_LIMIT'
  | 'OPENSCAD_RESIZE_BAD_NEWSIZE'
  | 'OPENSCAD_RESIZE_ZERO_EXTENT'
  | 'OPENSCAD_ROTATE_EXTRUDE_ANGLE_DEFAULTED'
  | 'OPENSCAD_ROTATE_EXTRUDE_FRAGMENT_PARAMETER'
  | 'OPENSCAD_ROTATE_EXTRUDE_FRAGMENTS_CLAMPED'

export interface OpenScadStableGeometryWarning {
  readonly code: OpenScadStableGeometryWarningCode
  readonly message: string
  readonly value?: unknown
  readonly limit?: number
}

export interface OpenScadStableGeometryContext {
  warn(warning: OpenScadStableGeometryWarning): void
}

export const SILENT_CONTEXT: OpenScadStableGeometryContext = Object.freeze({ warn() {} })

export const OPENSCAD_LINEAR_EXTRUDE_DEFAULT_HEIGHT = 100
export const OPENSCAD_LINEAR_EXTRUDE_MAX_SLICES = 512

export interface OpenScadLinearExtrudeResolutionInput {
  readonly height?: unknown
  readonly scale?: unknown
  readonly center?: unknown
  readonly twist?: unknown
  readonly slices?: unknown
  readonly fn?: unknown
  readonly fa?: unknown
  readonly fs?: unknown
  /** Vertices after 2D child union, used by OpenSCAD's automatic slice rule. */
  readonly profilePoints?: readonly OpenScadPoint2[]
  /** Independent-engine safety bound; not an upstream language limit. */
  readonly maximumSlices?: number
}

export type OpenScadLinearExtrudeSliceSource =
  | 'explicit'
  | 'twist-$fn'
  | 'twist-$fa/$fs'
  | 'twist-minimum'
  | 'nonuniform-$fn'
  | 'nonuniform-$fs'
  | 'single'
  | 'empty'

export interface OpenScadLinearExtrudeResolution {
  readonly empty: boolean
  readonly height: number
  readonly scale: OpenScadScale2
  readonly center: boolean
  readonly twist: number
  /** OpenSCAD vertical intervals. */
  readonly slices: number
  readonly unboundedSlices: number
  readonly sliceSource: OpenScadLinearExtrudeSliceSource
  /** Manifold counts only extra interior copies, hence slices - 1. */
  readonly manifoldNDivisions: number
  readonly maximumSlices: number
  readonly reduced: boolean
}

export type OpenScadAffine2dMatrix = readonly [
  number, number, number,
  number, number, number,
  number, number, number,
]

export interface OpenScadMinkowski2dKernel<Section, Solid> {
  /** Must return a point contained in the section (a contour vertex is sufficient). */
  readonly anchor: (section: Section) => OpenScadPoint2
  readonly translate: (section: Section, offset: OpenScadPoint2) => Section
  readonly extrudeUnitPrism: (section: Section) => Solid
  readonly minkowskiSum: (left: Solid, right: Solid) => Solid
  readonly projectTo2d: (solid: Solid) => Section
}

export interface OpenScadChildrenSelectionInput {
  /** Distinguishes children() from children(undef). */
  readonly provided: boolean
  readonly value?: unknown
  readonly childCount: number
  readonly maximumRangeItems?: number
}

export interface OpenScadResizeResolutionInput {
  readonly dimension: 2 | 3
  readonly extents: readonly number[]
  readonly newsize: unknown
  readonly auto?: unknown
}

export interface OpenScadResizeResolution {
  readonly scales: readonly number[]
  readonly applied: boolean
  readonly valid: boolean
}

export interface OpenScadRotateExtrudeResolutionInput {
  readonly angle?: unknown
  readonly profileXMin: number
  readonly profileXMax: number
  readonly fn?: unknown
  readonly fa?: unknown
  readonly fs?: unknown
  readonly quality?: 'preview' | 'full'
  readonly maximumFragments?: number
}

export interface OpenScadRotateExtrudeResolution {
  readonly empty: boolean
  readonly crossesAxis: boolean
  readonly angle: number
  readonly profileSide: 'positive' | 'negative' | 'axis' | 'crossing'
  readonly reflectProfileX: boolean
  readonly postRotateDegrees: 0 | 180
  readonly circularSegments: number
  readonly reduced: boolean
}
