import type { MeshData } from './mesh'

/** Tessellation level requested from the geometry build pipeline. */
export type GeometryQuality = 'preview' | 'full'

/** Compiler-owned timings; queue/transport time belongs to the coordinator. */
export interface GeometryPhaseTimings {
  parseMs: number
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
export interface GeometryEvaluationResult {
  meshes: MeshData[]
  warnings: string[]
  volume: number
  surfaceArea: number
  quality: GeometryQuality
  /** True iff the requested quality actually changed evaluated geometry. */
  reduced: boolean
  timings: GeometryPhaseTimings
}
