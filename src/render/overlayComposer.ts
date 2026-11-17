/**
 * Selection and source-highlight overlay buffers, split out of the renderer
 * so slot ownership, target matching and release can be tested with a fake
 * device.
 */
import {
  pointForOverlay,
  rebuildSelectionOverlaySlots,
  rebuildSourceHighlightOverlaySlots,
  releaseOverlaySlot,
} from './overlayBuilder'
import type { PickHit, SelectionMode } from './rendererContracts'
import type { GMesh, OverlaySlot } from './types'

const emptySlot = (): OverlaySlot => ({ buffer: null, capacity: 0, count: 0 })

/** True when two hits address the same overlay target under the given selection mode. */
export function sameOverlayTarget(
  selectionMode: SelectionMode,
  meshes: readonly GMesh[],
  a: PickHit | null,
  b: PickHit | null,
): boolean {
  if (!a || !b || a.meshIndex !== b.meshIndex) return false
  if (selectionMode === 'object') return true
  if (selectionMode === 'face') {
    return a.faceId !== null && b.faceId !== null ? a.faceId === b.faceId : a.triangleIndex === b.triangleIndex
  }
  const ap = pointForOverlay(meshes, a)
  const bp = pointForOverlay(meshes, b)
  if (!ap || !bp) return false
  const scale = Math.max(1, ...ap.map(Math.abs), ...bp.map(Math.abs))
  return Math.hypot(ap[0] - bp[0], ap[1] - bp[1], ap[2] - bp[2]) <= scale * 1e-7
}

export interface SelectionOverlayInput {
  readonly dev: GPUDevice | null
  readonly selectionMode: SelectionMode
  readonly meshes: readonly GMesh[]
  readonly boundsRadius: number | undefined
  readonly selectedHit: PickHit | null
  readonly hoveredHit: PickHit | null
  readonly isMeshVisible: (index: number) => boolean
}

export interface SourceHighlightInput {
  readonly dev: GPUDevice | null
  readonly sourceHighlightId: number | null
  readonly meshes: readonly GMesh[]
  readonly isMeshVisible: (index: number) => boolean
}

/** Owns the overlay vertex slots drawn after the main pass. */
export class OverlayComposer {
  readonly selectionFaceSlot = emptySlot()
  readonly selectionLineSlot = emptySlot()
  readonly deepSelectionLineSlot = emptySlot()
  readonly sourceFaceSlot = emptySlot()
  readonly sourceLineSlot = emptySlot()

  rebuildSelection(input: SelectionOverlayInput): void {
    rebuildSelectionOverlaySlots({
      ...input,
      sameOverlayTarget: (a, b) => sameOverlayTarget(input.selectionMode, input.meshes, a, b),
      selectionFaceSlot: this.selectionFaceSlot,
      selectionLineSlot: this.selectionLineSlot,
      deepSelectionLineSlot: this.deepSelectionLineSlot,
    })
  }

  rebuildSourceHighlight(input: SourceHighlightInput): void {
    rebuildSourceHighlightOverlaySlots({
      ...input,
      sourceFaceSlot: this.sourceFaceSlot,
      sourceLineSlot: this.sourceLineSlot,
    })
  }

  clearSelection(): void {
    releaseOverlaySlot(this.selectionFaceSlot)
    releaseOverlaySlot(this.selectionLineSlot)
    releaseOverlaySlot(this.deepSelectionLineSlot)
  }

  clearSourceHighlight(): void {
    releaseOverlaySlot(this.sourceFaceSlot)
    releaseOverlaySlot(this.sourceLineSlot)
  }
}
