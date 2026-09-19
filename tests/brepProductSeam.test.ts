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
    expect(brepCapability('nurbs-ss-bezier-le3/1')?.maturity).toBe('AnalyticComplete')
    expect(brepCapability('analytic-boolean/1')?.maturity).toBe('Qualified')
    expect(brepCapability('nurbs-boolean-bezier-le3/1')?.maturity).toBe('Unavailable')
    expect(brepCapability('analytic-chamfer/1')?.maturity).toBe('AnalyticComplete')
    expect(brepCapability('iges-interchange/1')?.maturity).toBe('ResearchOnly')
    expect(brepCapability('analytic-fillet/1')?.maturity).toBe('ResearchOnly')
    expect(brepCapability('step-interchange/1')?.maturity).toBe('AnalyticComplete')
    expect(brepCapability('nurbs-step-bicubic-face/1')?.maturity).toBe('Qualified')
    expect(brepCapability('nurbs-step-trimmed-bicubic/1')?.maturity).toBe('Qualified')
    expect(brepCapability('nurbs-step-solid/1')?.maturity).toBe('Qualified')
    for (const id of [
      'numeric-evidence-curved-brep/1',
      'boundary-correspondence/1',
      'exact-sew/1',
      'global-solid-audit/1',
      'persistent-naming/1',
      'nurbs-boolean-bezier-le3/2',
      'nurbs-boolean-bezier-le3/3',
      'step-interchange/2',
      'analytic-multi-edge-fillet/1',
      'exact-parallel-frame-sweep/1',
      'certified-brep-tessellation/1',
      'certified-mass-properties/1',
      'step-interchange/3',
    ]) expect(brepCapability(id)?.maturity, id).toBe('Qualified')
    expect(brepCapability('authorized-heal-gap-le1/1')?.maturity).toBe('Unavailable')
    expect(brepCapability('authorized-heal-gap-le1/2')?.maturity).toBe('Qualified')
    expect(() => assertBrepCapabilityAllowsTopology('step-interchange/3')).toThrow(
      /does not permit topology change/,
    )
    expect(() => assertBrepCapabilityAllowsTopology('nurbs-ss-bezier-le3/1')).toThrow(
      /does not permit topology change/,
    )
    expect(() => assertBrepCapabilityAllowsTopology('analytic-boolean/1')).not.toThrow()
    expect(() => assertBrepCapabilityAllowsTopology('nurbs-boolean-bezier-le3/1')).toThrow(
      /Unavailable; refuse topology change/,
    )
    expect(() => assertBrepCapabilityAllowsTopology('nurbs-boolean-bezier-le3/2')).not.toThrow()
    expect(() => assertBrepCapabilityAllowsTopology('nurbs-boolean-bezier-le3/3')).not.toThrow()
    expect(() => assertBrepCapabilityAllowsTopology('analytic-multi-edge-fillet/1')).not.toThrow()
    expect(() => assertBrepCapabilityAllowsTopology('exact-parallel-frame-sweep/1')).not.toThrow()
    expect(() => assertBrepCapabilityAllowsTopology('planar-csg/1')).not.toThrow()
    expect(BREP_CAPABILITY_MATRIX.some(c => c.id === 'iges-interchange/1')).toBe(true)
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
    const a = topoIdFromParts(1, 1, 'face')
    const b = topoIdFromParts(1, 2, 'face')
    const c = topoIdFromParts(1, 3, 'face')
    service.introduce(a, 'face')
    expect(service.transfer(a)).toEqual({ status: 'persistent', id: a })
    service.split(a, [b, c])
    expect(service.transfer(a)).toEqual({ status: 'confirmation-required', ids: [b, c] })
    expect(service.transfer(topoIdFromParts(9, 9, 'face'))).toEqual({ status: 'lost' })
  })
})
