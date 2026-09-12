import { describe, expect, it } from 'vitest'
import {
  classifyModel,
  compareSquaredDistance,
  coerceIndeterminateForbidden,
  orient2d,
  orient3d,
  rat,
  validateToleranceContext,
  type ToleranceContext,
} from './support/referencePredicateOracleV1'

const ctx: ToleranceContext = {
  linear_abs: 1e-9,
  linear_rel: 1e-12,
  on_tol: 1e-6,
  clear_tol: 1e-4,
  angular: 1e-9,
  param_floor: 1e-12,
  ulp_guard: 8,
  max_entity_error: 1e-3,
  policy: 'g2a-box',
}

describe('G0.7 predicate oracle', () => {
  it('orient2d returns exact signs for rational leaves', () => {
    expect(orient2d([0, 0], [1, 0], [0, 1])).toBe('Positive')
    expect(orient2d([0, 0], [1, 0], [0, -1])).toBe('Negative')
    expect(orient2d([0, 0], [1, 0], [2, 0])).toBe('Zero')
    expect(orient2d([rat(0), rat(0)], [rat(1, 2), rat(0)], [rat(0), rat(1, 3)])).toBe(
      'Positive',
    )
  })

  it('orient3d signs a unit tetrahedron and coplanar points', () => {
    expect(orient3d([0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1])).toBe('Positive')
    expect(orient3d([0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, -1])).toBe('Negative')
    expect(orient3d([0, 0, 0], [1, 0, 0], [0, 1, 0], [1, 1, 0])).toBe('Zero')
  })

  it('compareSquaredDistance distinguishes inside/on/outside a sphere', () => {
    expect(compareSquaredDistance([0, 0, 0], [1, 0, 0], 1)).toBe('Zero')
    expect(compareSquaredDistance([0, 0, 0], [0.5, 0, 0], 1)).toBe('Negative')
    expect(compareSquaredDistance([0, 0, 0], [2, 0, 0], 1)).toBe('Positive')
  })

  it('rejects invalid ToleranceContext and keeps gray-band Indeterminate', () => {
    expect(validateToleranceContext({ ...ctx, on_tol: 0 })).toBe('on_tol_clear_tol')
    expect(validateToleranceContext({ ...ctx, clear_tol: ctx.on_tol })).toBe(
      'on_tol_clear_tol',
    )
    expect(classifyModel(5e-5, 1e-8, ctx)).toBe('Indeterminate')
    expect(classifyModel(1e-9, 1e-12, ctx)).toBe('Coincident')
    expect(classifyModel(1e-3, 1e-8, ctx)).toBe('Separate')
    expect(classifyModel(1e-9, null, ctx)).toBe('Indeterminate')
  })

  it('forbids coercing Indeterminate to success', () => {
    expect(() => coerceIndeterminateForbidden('Indeterminate')).toThrow(/Indeterminate/)
    expect(() => coerceIndeterminateForbidden('Zero')).not.toThrow()
  })

  it('returns Indeterminate for non-finite binary64 leaves', () => {
    expect(orient2d([0, 0], [1, 0], [Number.NaN, 0])).toBe('Indeterminate')
    expect(orient3d([0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, Number.POSITIVE_INFINITY])).toBe(
      'Indeterminate',
    )
  })
})
