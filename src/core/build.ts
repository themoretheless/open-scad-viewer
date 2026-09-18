import type { MeshData } from './mesh'

/** Tessellation level requested from the geometry build pipeline. */
export type GeometryQuality = 'preview' | 'full'

/** Compiler-owned timings; queue/transport time belongs to the coordinator. */
export interface GeometryPhaseTimings {
  parseMs: number
  bindMs: number
  initializeMs: number
  evaluateMs: number
  analyzeMs: number
}

/**
 * Source- and kernel-neutral terminal geometry result used at the engine
 * facade boundary. Legacy callers may still refer to the parser's
 * `ParseResult` alias, but alternate evaluators must not depend on the parser
 * module merely to publish the same browser-safe result shape.
 */
/**
 * The exact solid graph recorded beside a polygon evaluation, so the Solid workspace
 * can build true NURBS bodies from the same source the Mesh workspace shows.
 *
 * Structural on purpose: an alternate evaluator may fill it without depending on the
 * parser, and it crosses the worker boundary as plain data.
 */
export interface ExactSolidPlan {
  nodes: ReadonlyArray<Record<string, unknown> & { id: string; op: string }>
  /** One entry per displayed solid, in mesh order. */
  roots: ReadonlyArray<{ name: string; id: string } | { name: string; inexact: string }>
}

export interface GeometryEvaluationResult {
  meshes: MeshData[]
  /** Absent when the evaluator does not record exact solids. */
  exactSolids?: ExactSolidPlan
  warnings: string[]
  volume: number
  surfaceArea: number
  quality: GeometryQuality
  /** True iff the requested quality actually changed evaluated geometry. */
  reduced: boolean
  timings: GeometryPhaseTimings
}
