import type { GeometryQuality } from '../core/build'

/**
 * Version of every quality-dependent geometry decision covered by `reduced`.
 * Increment this when preview/full can diverge for a new reason and extend the
 * equivalence corpus before allowing promotion under the new policy.
 */
export const BUILD_EQUIVALENCE_POLICY_VERSION = 1

export type GeometryPublicationPlan =
  | { publish: false; effectiveQuality: null; requestFull: false; promoted: false }
  | { publish: true; effectiveQuality: GeometryQuality; requestFull: boolean; promoted: boolean }

export interface GeometryPublicationCandidate {
  buildGeneration: number
  source: string
  quality: GeometryQuality
  reduced: boolean
}

export interface CurrentGeometryTarget {
  buildGeneration: number
  source: string
}

export type LastKnownGoodDisplayState =
  | 'missing'
  | 'building'
  | 'last-known-good-stale'
  | 'preview-only'
  | 'current-exportable'

export interface GeometryExportCandidate {
  meshCount: number
  rendering: boolean
  hasError: boolean
  renderedQuality: GeometryQuality | null
  renderedSource: string
  currentSource: string
}

export interface GeometryExportEligibility {
  allowed: boolean
  state: LastKnownGoodDisplayState
  interaction: 'current' | 'read-only'
}

/** A retained prior mesh is always read-only after failure or source drift. */
export function geometryExportEligibility(
  candidate: GeometryExportCandidate,
): GeometryExportEligibility {
  if (candidate.meshCount === 0 || candidate.renderedSource === '') {
    return { allowed: false, state: 'missing', interaction: 'read-only' }
  }
  if (candidate.rendering) {
    return { allowed: false, state: 'building', interaction: 'read-only' }
  }
  if (candidate.hasError || candidate.renderedSource !== candidate.currentSource) {
    return { allowed: false, state: 'last-known-good-stale', interaction: 'read-only' }
  }
  if (candidate.renderedQuality !== 'full') {
    return { allowed: false, state: 'preview-only', interaction: 'read-only' }
  }
  return { allowed: true, state: 'current-exportable', interaction: 'current' }
}

/** Pure preview→full policy; Vue owns no quality inference. */
export function planGeometryPublication(
  candidate: GeometryPublicationCandidate,
  current: CurrentGeometryTarget,
): GeometryPublicationPlan {
  if (candidate.buildGeneration !== current.buildGeneration || candidate.source !== current.source) {
    return { publish: false, effectiveQuality: null, requestFull: false, promoted: false }
  }
  if (candidate.quality === 'full') {
    return { publish: true, effectiveQuality: 'full', requestFull: false, promoted: false }
  }
  if (candidate.reduced) {
    return { publish: true, effectiveQuality: 'preview', requestFull: true, promoted: false }
  }
  return { publish: true, effectiveQuality: 'full', requestFull: false, promoted: true }
}
