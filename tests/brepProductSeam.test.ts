import { describe, expect, it } from 'vitest'
import {
  BREP_CAPABILITY_MATRIX,
  assertBrepCapabilityAllowsTopology,
  brepCapability,
} from '../src/services/geometry/brepCapability'
import {
  cancelGeometryKernel,
  issueGeometryLease,
  publishLastKnownGood,
  readLastKnownGood,
  resetGeometryKernelCancel,
  assertGeometryLeaseCurrent,
  GeometryLeaseError,
} from '../src/services/geometry/kernelLeases'
import { createSelectionTransferService } from '../src/services/geometry/selectionTransfer'
import { topoIdFromParts } from '../src/core/topologyLineage'

describe('B-rep product seam', () => {
  it('publishes honest capability maturity without fake Available', () => {
    expect(BREP_CAPABILITY_MATRIX.some(c => c.id === 'analytic-boolean/1')).toBe(true)
    expect(brepCapability('nurbs-ss-transverse-bicubic/1')?.maturity).toBe('AnalyticComplete')
    expect(brepCapability('analytic-boolean/1')?.maturity).toBe('Qualified')
    expect(brepCapability('analytic-fillet/1')?.maturity).toBe('AnalyticComplete')
    expect(brepCapability('step-interchange/1')?.maturity).toBe('AnalyticComplete')
    expect(() => assertBrepCapabilityAllowsTopology('nurbs-ss-transverse-bicubic/1')).toThrow(
      /does not permit topology change/,
    )
    expect(() => assertBrepCapabilityAllowsTopology('analytic-boolean/1')).not.toThrow()
    expect(() => assertBrepCapabilityAllowsTopology('planar-csg/1')).not.toThrow()
  })

  it('cancels leases and marks LKG stale', () => {
    resetGeometryKernelCancel()
    const lease = issueGeometryLease('test')
    assertGeometryLeaseCurrent(lease)
    const lkg = publishLastKnownGood({ ok: true })
    expect(readLastKnownGood(lkg)).toEqual({ ok: true })
    cancelGeometryKernel()
    expect(() => issueGeometryLease('test')).toThrow(GeometryLeaseError)
    expect(() => assertGeometryLeaseCurrent(lease)).toThrow(GeometryLeaseError)
    expect(() => readLastKnownGood(lkg)).toThrow(GeometryLeaseError)
    resetGeometryKernelCancel()
  })

  it('transfers selection without nearest-face guessing', () => {
    const service = createSelectionTransferService()
    const a = topoIdFromParts(1, 1)
    const b = topoIdFromParts(1, 2)
    const c = topoIdFromParts(1, 3)
    service.introduce(a, 'face')
    expect(service.transfer(a)).toEqual({ status: 'persistent', id: a })
    service.split(a, [b, c])
    expect(service.transfer(a)).toEqual({ status: 'ambiguous', ids: [b, c] })
    expect(service.transfer(topoIdFromParts(9, 9))).toEqual({ status: 'lost' })
  })
})
