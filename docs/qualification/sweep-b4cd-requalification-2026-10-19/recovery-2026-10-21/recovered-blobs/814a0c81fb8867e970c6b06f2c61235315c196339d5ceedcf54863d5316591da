import type { MeshData } from '../core/mesh'
import { matchMeshesByProvenance } from './meshInspection'

export interface ScenePublicationInput {
  previousMeshes: readonly MeshData[]
  previousVisibility: readonly boolean[]
  previousSelectedIndex: number | null
  previousIsolated: boolean
  nextMeshes: readonly MeshData[]
  /** True only for two tessellations of the exact same source snapshot. */
  sameSourceSnapshot: boolean
}

export interface ScenePublicationPlan {
  /** Previous mesh index for each next mesh, or -1 when identity is not safe. */
  previousIndexByNextIndex: number[]
  /** Visibility for the next scene. New or ambiguous objects start visible. */
  nextVisibility: boolean[]
  /** The selected mesh in the next scene, if its identity survived publication. */
  nextSelectedIndex: number | null
  /** Isolation survives only together with an identity-safe selection. */
  nextIsolated: boolean
  /** World-space measurement overlays are valid only for the same source snapshot. */
  measurementMayBePreserved: boolean
}

/**
 * Compute the renderer-agnostic state transition for publishing a replacement
 * scene. Identity ambiguity deliberately resets state instead of attaching it
 * to a potentially different object.
 */
export function planScenePublication(input: ScenePublicationInput): ScenePublicationPlan {
  const previousIndexByNextIndex = input.previousMeshes.length === 0
    ? input.nextMeshes.map(() => -1)
    : matchMeshesByProvenance(input.previousMeshes, input.nextMeshes, {
        sameSourceSnapshot: input.sameSourceSnapshot,
      })

  const nextVisibility = previousIndexByNextIndex.map(previousIndex => (
    previousIndex >= 0 ? input.previousVisibility[previousIndex] !== false : true
  ))

  const previousSelectedIndex = isValidSelection(
    input.previousSelectedIndex,
    input.previousMeshes.length,
  )
    ? input.previousSelectedIndex
    : null
  const replacementSelection = previousSelectedIndex === null
    ? -1
    : previousIndexByNextIndex.indexOf(previousSelectedIndex)
  // Renderer visibility changes clear hidden selections. Preserve the same
  // invariant when a replacement scene is published, before isolation is
  // restored and can otherwise produce an apparently empty viewport.
  const nextSelectedIndex = replacementSelection >= 0 && nextVisibility[replacementSelection] !== false
    ? replacementSelection
    : null

  return {
    previousIndexByNextIndex,
    nextVisibility,
    nextSelectedIndex,
    nextIsolated: nextSelectedIndex !== null && input.previousIsolated,
    measurementMayBePreserved: input.sameSourceSnapshot,
  }
}

function isValidSelection(index: number | null, meshCount: number): index is number {
  return index !== null && Number.isInteger(index) && index >= 0 && index < meshCount
}
