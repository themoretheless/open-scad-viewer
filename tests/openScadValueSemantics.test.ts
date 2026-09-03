import { describe, expect, it } from 'vitest'
import { TT, type RangeValue, type Value } from '../src/services/openscadCompiler'
import {
  formatOpenScadValue,
  materializeOpenScadRange,
  openScadBinary,
  openScadIndex,
  openScadTruthy,
  openScadUnary,
  type OpenScadValueSemanticsContext,
} from '../src/services/openScadValueSemantics'

function harness() {
  const warnings: string[] = []
  const context: OpenScadValueSemanticsContext = {
    warn: warning => warnings.push(warning),
    maxRangeItems: 10_000,
  }
  return { warnings, context }
}

function binary(operator: TT, left: Value, right: Value) {
  const { warnings, context } = harness()
  return { value: openScadBinary(operator, left, right, context), warnings }
}

describe('shared OpenSCAD 2021.01 value semantics', () => {
  it('implements scalar, vector and matrix arithmetic observed from the oracle', () => {
    expect(binary(TT.Plus, [1, 2], [3, 4]).value).toEqual([4, 6])
    expect(binary(TT.Star, [1, 2], [3, 4]).value).toBe(11)
    expect(binary(TT.Star, 2, [3, 4]).value).toEqual([6, 8])
    expect(binary(TT.Star, [[1, 2], [3, 4]], [5, 6]).value).toEqual([17, 39])
    expect(binary(TT.Star, [5, 6], [[1, 2], [3, 4]]).value).toEqual([23, 34])
    expect(binary(TT.Star, [[1, 2], [3, 4]], [[5, 6], [7, 8]]).value)
      .toEqual([[19, 22], [43, 50]])
    expect(binary(TT.Slash, [6, 8], 2).value).toEqual([3, 4])
    expect(binary(TT.Slash, 12, [3, 4]).value).toEqual([4, 3])
    expect(openScadUnary(TT.Minus, [1, 2], harness().context)).toEqual([-1, -2])
  })

  it('returns undef plus a warning for undefined vector operations', () => {
    const mismatch = binary(TT.Star, [1, 2], [1])
    expect(mismatch.value).toBeUndefined()
    expect(mismatch.warnings).toContain('vector*vector requires matching lengths (2 != 1)')

    const invalid = binary(TT.Plus, [1, 2], 1)
    expect(invalid.value).toBeUndefined()
    expect(invalid.warnings).toContain('Undefined operation (vector + number)')
  })

  it('keeps ranges tagged, materializes them only on demand, and indexes Unicode by code point', () => {
    const range: RangeValue = { kind: 'range-value', start: 0, step: 1, end: 2 }
    const { context } = harness()
    expect(formatOpenScadValue(range)).toBe('[0 : 1 : 2]')
    expect(materializeOpenScadRange(range, context)).toEqual([0, 1, 2])
    expect(openScadIndex(range, 1.9, context)).toBe(1)
    expect(openScadIndex([10, 20], 1.9, context)).toBe(20)
    expect(openScadIndex('💩x', 0, context)).toBe('💩')
    expect(openScadIndex('💩x', 1, context)).toBe('x')
    expect(openScadTruthy({ kind: 'range-value', start: 1, step: 1, end: 0 })).toBe(true)
  })

  it('preserves inf/nan values instead of turning them into fatal evaluator errors', () => {
    expect(binary(TT.Slash, 1, 0).value).toBe(Infinity)
    expect(binary(TT.Slash, -1, 0).value).toBe(-Infinity)
    expect(Number.isNaN(binary(TT.Slash, 0, 0).value)).toBe(true)
    expect(Number.isNaN(binary(TT.Percent, 1, 0).value)).toBe(true)
    expect(formatOpenScadValue(Number.NaN)).toBe('nan')
    expect(formatOpenScadValue(1 / 3)).toBe('0.333333')
    expect(formatOpenScadValue(0.000000123456789)).toBe('1.23457e-7')
  })
})
