import { BREP_CAPABILITY_MATRIX } from '../brepCapability'
import type { BrepCapabilityDescriptor } from '../brepCapability'

export function brepCapability(id: string): BrepCapabilityDescriptor | undefined {
  return BREP_CAPABILITY_MATRIX.find(entry => entry.id === id)
}

export function assertBrepCapabilityAllowsTopology(id: string): void {
  const entry = brepCapability(id)
  if (!entry) throw new Error(`Unknown B-rep capability: ${id}`)
  if (entry.maturity === 'Unavailable' || entry.maturity === 'ResearchOnly') {
    throw new Error(`B-rep capability ${id} is ${entry.maturity}; refuse topology change`)
  }
  if (!entry.permitsTopologyChange) {
    throw new Error(`B-rep capability ${id} does not permit topology change`)
  }
}

/** Rollback drill: revoked / stale LKG must not fall through to Manifold. */
export function assertNoManifoldFallbackOnBrepFailure(capabilityId: string): void {
  const entry = brepCapability(capabilityId)
  if (!entry) throw new Error(`Unknown B-rep capability: ${capabilityId}`)
  if (entry.maturity === 'Unavailable') {
    throw new Error(`B-rep capability ${capabilityId} unavailable; refuse Manifold cross-route`)
  }
}
