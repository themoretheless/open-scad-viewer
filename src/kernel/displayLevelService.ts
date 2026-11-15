/**
 * Certified B-rep display levels computed on demand, away from scene
 * publication. The viewer asks for a level summary first (triangle counts and
 * certified errors drive LOD selection) and then for the arrays of the one
 * level the display scheduler wants. Computed level sets are kept per
 * geometry key in a small LRU so successive level requests reuse one native
 * evaluation.
 */
import type { MeshBvh } from './meshBvh'
import { buildMeshBvh } from './meshBvh'
import { extractSemanticEdges } from './meshTopology'
import { prepareBrepDisplayLevels } from './geometry/brep'

/** Levels whose finest mesh exceeds this stay unpublished; the single base mesh is kept. */
export const MAX_LOD_TRIANGLES = 20_000

export interface DisplayLevelSummaryEntry {
  readonly triangleCount: number
  readonly vertexCount: number
  /** Certified two-sided deviation from the exact surface, model units. */
  readonly errorUpper: number
}

export interface DisplayLevelPayload {
  readonly vertices: Float32Array
  readonly indices: Uint32Array
  readonly edgeIndices: Uint32Array
  readonly faceIds: Uint32Array
  readonly bvh: MeshBvh
}

export interface DisplayLevelData extends DisplayLevelPayload, DisplayLevelSummaryEntry {}

type LevelEvaluator = (model: unknown) => DisplayLevelData[] | null

/** Native evaluation of every certified level of one B-rep model, or null when it has none. */
export const evaluateBrepDisplayLevels: LevelEvaluator = model => {
  const levels = prepareBrepDisplayLevels(model as Parameters<typeof prepareBrepDisplayLevels>[0])
  if (!levels || levels.length < 2 || levels.length > 3) return null
  if (levels.some(level => !level.closed || level.triangleCount > MAX_LOD_TRIANGLES)) return null
  return levels.map(level => {
    const vertices = new Float32Array(level.displayVertices)
    const indices = new Uint32Array(level.displayIndices)
    return {
      vertices,
      indices,
      edgeIndices: extractSemanticEdges(vertices, indices).indices,
      faceIds: new Uint32Array(level.faceIds),
      bvh: buildMeshBvh(vertices, indices),
      triangleCount: level.triangleCount,
      vertexCount: vertices.length / 6,
      errorUpper: level.errorUpper,
    }
  })
}

/** The B-rep model carried by a native geometry snapshot's `geometryJson`. */
export function brepModelFromGeometryJson(geometryJson: string): unknown {
  const parsed = JSON.parse(geometryJson) as { geometry?: unknown }
  if (!parsed || typeof parsed !== 'object' || !parsed.geometry) throw new Error('Snapshot carries no B-rep model')
  return parsed.geometry
}

export class DisplayLevelService {
  private readonly sets = new Map<string, DisplayLevelData[] | null>()

  constructor(
    private readonly evaluate: LevelEvaluator = evaluateBrepDisplayLevels,
    private readonly capacity = 4,
  ) {}

  summary(geometry: string, geometryJson: string): DisplayLevelSummaryEntry[] | null {
    const levels = this.levels(geometry, geometryJson)
    return levels
      ? levels.map(({ triangleCount, vertexCount, errorUpper }) => ({ triangleCount, vertexCount, errorUpper }))
      : null
  }

  level(geometry: string, geometryJson: string, index: number): DisplayLevelData {
    const level = this.levels(geometry, geometryJson)?.[index]
    if (!level) throw new RangeError(`Geometry ${geometry} has no display level ${index}`)
    return level
  }

  forget(geometry: string) {
    this.sets.delete(geometry)
  }

  private levels(geometry: string, geometryJson: string): DisplayLevelData[] | null {
    if (this.sets.has(geometry)) {
      const cached = this.sets.get(geometry)!
      this.sets.delete(geometry)
      this.sets.set(geometry, cached)
      return cached
    }
    let levels: DisplayLevelData[] | null
    try {
      levels = this.evaluate(brepModelFromGeometryJson(geometryJson))
    } catch {
      levels = null
    }
    this.sets.set(geometry, levels)
    while (this.sets.size > this.capacity) this.sets.delete(this.sets.keys().next().value!)
    return levels
  }
}
