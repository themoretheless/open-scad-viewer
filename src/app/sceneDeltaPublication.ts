/**
 * App scene publication over the renderer's scene-delta path. The renderer
 * keeps GPU buffers and interaction for entities that persist unchanged; this
 * module only reconciles the carried selection with the App's provenance plan
 * and drops publication intents from older generations.
 */
import type { MeshData } from '../core/mesh'
import type { PickHit } from '../render/rendererContracts'

/** Monotonic generation gate: an intent older than the newest accepted one is stale. */
export class SceneIntentGenerations {
  private newest = Number.NEGATIVE_INFINITY

  /** Accept an intent tagged with `generation`; false when a newer one was already accepted. */
  accept(generation: number): boolean {
    if (!Number.isFinite(generation) || generation < this.newest) return false
    this.newest = generation
    return true
  }

  isCurrent(generation: number): boolean {
    return generation === this.newest
  }

  get current(): number | null {
    return Number.isFinite(this.newest) ? this.newest : null
  }
}

/** Renderer surface the reconciliation reads after a delta publication. */
export interface CarriedSelectionTarget {
  readonly selectedIndex: number | null
  readonly isIsolated: boolean
  selectMesh(index: number | null): void
  toggleIsolateSelection(): boolean
  restoreNativeFaceSelection(previous: MeshData, hit: PickHit, index: number): boolean
}

export interface SelectionPlan {
  readonly nextSelectedIndex: number | null
  readonly nextIsolated: boolean
}

export type CarriedSelectionOutcome = 'carried' | 'restored' | 'cleared' | 'none'

/**
 * Apply the App plan on top of what the renderer carried. When the renderer
 * already holds the planned selection and isolation (a stable entity), nothing
 * is re-applied, so the face hit and camera interaction survive the republish.
 */
export function reconcileCarriedSelection(
  target: CarriedSelectionTarget,
  plan: SelectionPlan,
  previousFace: { readonly mesh: MeshData; readonly hit: PickHit } | null,
): CarriedSelectionOutcome {
  const planned = plan.nextSelectedIndex
  if (planned === null) {
    if (target.selectedIndex === null) return 'none'
    target.selectMesh(null)
    return 'cleared'
  }
  if (target.selectedIndex === planned && target.isIsolated === plan.nextIsolated) return 'carried'
  if (target.selectedIndex !== planned) {
    const faceRestored = previousFace
      ? target.restoreNativeFaceSelection(previousFace.mesh, previousFace.hit, planned)
      : false
    if (!faceRestored) target.selectMesh(planned)
  }
  if (target.isIsolated !== plan.nextIsolated) target.toggleIsolateSelection()
  return 'restored'
}
