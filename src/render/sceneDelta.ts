/**
 * Scene publication deltas. The renderer stages a new publication against the
 * current one (geometry reuse by asset id lives in `meshBufferManager` and
 * `gpuGeometryCache`); this module classifies entities by id and carries
 * interaction state (selection, hover, isolation, measurement) across
 * publications for entities that persist unchanged.
 */
import { sameTypedArray } from './drawSorting'
import type { PickHit } from './rendererContracts'
import type { MeshData } from '../core/mesh'
import type { GMesh } from './types'

/** What the diff reads from a staged mesh; `vb` is any identity of the uploaded geometry. */
export interface DeltaMesh {
  readonly entityId?: string | undefined
  readonly assetId?: string | undefined
  readonly vb: object
  readonly transform: Float32Array
  readonly faceIds: Uint32Array
  readonly nativeGeometry?: GMesh['nativeGeometry']
}

/** Delta view of an unstaged mesh: the vertex array stands in for the GPU buffer. */
export const deltaMeshFromMeshData = (mesh: MeshData): DeltaMesh => ({
  entityId: mesh.entityId,
  assetId: mesh.geometryAssetId,
  vb: mesh.vertices,
  transform: mesh.transform,
  faceIds: mesh.faceIds,
  nativeGeometry: mesh.nativeGeometry,
})

export interface SceneDelta {
  /** Previous index to next index for entities with the same id, geometry and transform. */
  readonly stable: ReadonlyMap<number, number>
  /** Next indices of entities with no previous counterpart (or no entity id). */
  readonly added: readonly number[]
  /** Previous indices of entities absent from the next publication. */
  readonly removed: readonly number[]
  /** Next indices of persisting entities whose geometry changed. */
  readonly geometryChanged: readonly number[]
  /** Next indices of persisting entities with unchanged geometry and a new transform. */
  readonly transformChanged: readonly number[]
}

/**
 * Same geometry: equal content-addressed asset ids (the same uploaded vertex
 * buffer when either side has none), equal face ids and the same native B-rep snapshot, so face hits stay valid.
 */
function sameGeometry(previous: DeltaMesh, next: DeltaMesh) {
  const a = previous.nativeGeometry,
    b = next.nativeGeometry
  const sameAsset =
    previous.assetId !== undefined && next.assetId !== undefined
      ? previous.assetId === next.assetId
      : previous.vb === next.vb
  return (
    sameAsset &&
    (previous.faceIds === next.faceIds || sameTypedArray(previous.faceIds, next.faceIds)) &&
    (a === b || (!!a && !!b && a.geometryJson === b.geometryJson && a.documentJson === b.documentJson))
  )
}

export function diffSceneEntities(previous: readonly DeltaMesh[], next: readonly DeltaMesh[]): SceneDelta {
  const previousById = new Map<string, number>()
  previous.forEach((mesh, index) => {
    if (mesh.entityId) previousById.set(mesh.entityId, index)
  })
  const stable = new Map<number, number>()
  const added: number[] = []
  const geometryChanged: number[] = []
  const transformChanged: number[] = []
  const persisted = new Set<number>()
  next.forEach((mesh, index) => {
    const previousIndex = mesh.entityId ? previousById.get(mesh.entityId) : undefined
    if (previousIndex === undefined || persisted.has(previousIndex)) {
      added.push(index)
      return
    }
    persisted.add(previousIndex)
    const old = previous[previousIndex]!
    if (!sameGeometry(old, mesh)) geometryChanged.push(index)
    else if (!sameTypedArray(old.transform, mesh.transform)) transformChanged.push(index)
    else stable.set(previousIndex, index)
  })
  const removed = previous.map((_, index) => index).filter(index => !persisted.has(index))
  return { stable, added, removed, geometryChanged, transformChanged }
}

/** True when every previous entity persists with unchanged geometry and transform. */
export const previousSceneIntact = (delta: SceneDelta, previousCount: number) => delta.stable.size === previousCount

export interface InteractionState {
  readonly selected: number | null
  readonly selectedHit: PickHit | null
  readonly hovered: number | null
  readonly hoveredHit: PickHit | null
  readonly isolated: boolean
}

const remapHit = (hit: PickHit | null, index: number | null): PickHit | null => {
  if (!hit || index === null) return null
  const { cycleIndex: _cycleIndex, cycleCount: _cycleCount, ...rest } = hit
  return { ...rest, meshIndex: index }
}

/**
 * Carry selection/hover to the new indices of stable entities. A target whose
 * entity left, changed geometry or moved is cleared; isolation needs a
 * surviving selection. Depth-cycle metadata never survives a publication.
 * A null delta clears everything (the caller owns re-selection).
 */
export function carryInteraction(state: InteractionState, delta: SceneDelta | null): InteractionState {
  const selected = state.selected === null ? null : (delta?.stable.get(state.selected) ?? null)
  const hovered = state.hovered === null ? null : (delta?.stable.get(state.hovered) ?? null)
  return {
    selected,
    selectedHit: remapHit(state.selectedHit, selected),
    hovered,
    hoveredHit: remapHit(state.hoveredHit, hovered),
    isolated: state.isolated && selected !== null,
  }
}

export function sameInteractionTarget(left: InteractionState, right: InteractionState, key: 'selected' | 'hovered') {
  const hitKey = key === 'selected' ? 'selectedHit' : 'hoveredHit'
  const a = left[hitKey],
    b = right[hitKey]
  return (
    left[key] === right[key] &&
    (key === 'hovered' || left.isolated === right.isolated) &&
    (a === b ||
      (!!a && !!b && a.meshIndex === b.meshIndex && a.triangleIndex === b.triangleIndex && a.faceId === b.faceId))
  )
}

/** Scene-delta carry for an unstaged publication (CPU-only renderers share the WebGPU rules). */
export function carryMeshPublication<M>(
  previous: readonly MeshData[],
  next: readonly MeshData[],
  state: Pick<InteractionState, 'selected' | 'selectedHit' | 'isolated'> & { readonly measurement: M | null },
  options: { readonly preserveInteraction?: boolean; readonly preserveMeasurement?: boolean },
): InteractionState & { measurement: M | null } {
  const delta = diffSceneEntities(previous.map(deltaMeshFromMeshData), next.map(deltaMeshFromMeshData))
  const keepMeasurement = options.preserveMeasurement ?? previousSceneIntact(delta, previous.length)
  const carried = { ...state, hovered: null, hoveredHit: null }
  return {
    ...carryInteraction(carried, options.preserveInteraction === false ? null : delta),
    measurement: keepMeasurement ? state.measurement : null,
  }
}
