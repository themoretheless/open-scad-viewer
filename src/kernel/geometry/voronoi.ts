import { callGeometryRust } from './kernel'

export type VoronoiPattern = 'voronoi' | 'web' | 'honeycomb' | 'isogrid' | 'triangles' | 'grid'
export type VoronoiGradientMode = 'none' | 'vertical' | 'radial'

export interface VoronoiCellsOptions {
  min: readonly [number, number]
  max: readonly [number, number]
  pattern?: VoronoiPattern
  cell: number
  rib: number
  seed?: number
  jitter?: number
  overhangAngle?: number | null
  stressGradient?: number
  gradientMode?: VoronoiGradientMode
  cornerRadius?: number
  slope?: number
  frame?: number
  keepOut?: ReadonlyArray<readonly [number, number, number, number]>
  maxSites?: number
}

/** Generate 2D Voronoi/honeycomb/isogrid/grid cell opening polygons via the Rust lattice kernel. */
export function generateVoronoiCells(options: VoronoiCellsOptions): Array<Array<[number, number]>> {
  return callGeometryRust('cad_voronoi_cells', options)
}
