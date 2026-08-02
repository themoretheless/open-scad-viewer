import { describe, expect, it } from 'vitest'
import {
  geometryExportEligibility,
  planGeometryPublication,
} from '../src/services/buildPromotionPolicy'

const current = { buildGeneration: 4, source: 'sphere(5);' }

describe('preview/full publication policy', () => {
  it('promotes an equivalent preview without requesting a duplicate full build', () => {
    expect(planGeometryPublication({ ...current, quality: 'preview', reduced: false }, current)).toEqual({
      publish: true, effectiveQuality: 'full', requestFull: false, promoted: true,
    })
  })

  it('publishes a reduced preview and requests exactly one full continuation', () => {
    expect(planGeometryPublication({ ...current, quality: 'preview', reduced: true }, current)).toEqual({
      publish: true, effectiveQuality: 'preview', requestFull: true, promoted: false,
    })
  })

  it('publishes a current full result without another continuation', () => {
    expect(planGeometryPublication({ ...current, quality: 'full', reduced: false }, current)).toEqual({
      publish: true, effectiveQuality: 'full', requestFull: false, promoted: false,
    })
  })

  it.each([
    { buildGeneration: 3, source: current.source },
    { buildGeneration: current.buildGeneration, source: 'cube(1);' },
  ])('ignores a result outside the exact current target: %o', candidate => {
    expect(planGeometryPublication({ ...candidate, quality: 'preview', reduced: false }, current)).toEqual({
      publish: false, effectiveQuality: null, requestFull: false, promoted: false,
    })
  })
})

describe('last-known-good export policy', () => {
  const full = {
    meshCount: 1,
    rendering: false,
    hasError: false,
    renderedQuality: 'full' as const,
    renderedSource: 'cube(1);',
    currentSource: 'cube(1);',
  }

  it('exports only a current error-free full result', () => {
    expect(geometryExportEligibility(full)).toEqual({
      allowed: true,
      state: 'current-exportable',
      interaction: 'current',
    })
  })

  it.each([
    [{ ...full, hasError: true }, 'last-known-good-stale'],
    [{ ...full, currentSource: 'sphere(1);' }, 'last-known-good-stale'],
    [{ ...full, rendering: true }, 'building'],
    [{ ...full, renderedQuality: 'preview' as const }, 'preview-only'],
    [{ ...full, meshCount: 0 }, 'missing'],
  ] as const)('keeps non-current geometry read-only: %s', (candidate, state) => {
    expect(geometryExportEligibility(candidate)).toEqual({
      allowed: false,
      state,
      interaction: 'read-only',
    })
  })
})
