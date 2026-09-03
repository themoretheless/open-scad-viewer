import { describe, expect, it } from 'vitest'
import {
  OPENSCAD_NAMED_COLOR_HEX,
  resolveOpenScadColor,
  resolveOpenScadFragments,
  resolveOpenScadOffset,
  resolveOpenScadSweepFragments,
  type OpenScadStableModuleWarning,
  type OpenScadStableModuleSemanticsContext,
} from '../src/services/openScadStableModuleSemantics'

function harness() {
  const warnings: OpenScadStableModuleWarning[] = []
  const context: OpenScadStableModuleSemanticsContext = {
    warn: warning => warnings.push(warning),
  }
  return { warnings, context }
}

describe('kernel-neutral OpenSCAD 2021 stable module semantics', () => {
  it('contains the complete case-insensitive CSS/SVG keyword set and transparent', () => {
    expect(Object.keys(OPENSCAD_NAMED_COLOR_HEX)).toHaveLength(149)
    expect(OPENSCAD_NAMED_COLOR_HEX).toMatchObject({
      aliceblue: 'F0F8FF',
      darkslategrey: '2F4F4F',
      rebeccapurple: '663399',
      transparent: '00000000',
      yellowgreen: '9ACD32',
    })
    for (const [name, hex] of Object.entries(OPENSCAD_NAMED_COLOR_HEX)) {
      expect(name).toBe(name.toLowerCase())
      expect(hex).toMatch(/^(?:[0-9A-F]{6}|[0-9A-F]{8})$/)
      expect(resolveOpenScadColor(name.toUpperCase())).toMatchObject({ valid: true, source: 'named' })
    }
    expect(resolveOpenScadColor('gray').rgba).toEqual([128 / 255, 128 / 255, 128 / 255, 1])
    expect(resolveOpenScadColor('grey').rgba).toEqual(resolveOpenScadColor('gray').rgba)
    expect(resolveOpenScadColor('transparent').rgba).toEqual([0, 0, 0, 0])
  })

  it('accepts all four OpenSCAD CSS hex widths without accepting near misses', () => {
    expect(resolveOpenScadColor('#abc')).toEqual({
      rgba: [0xaa / 255, 0xbb / 255, 0xcc / 255, 1], valid: true, source: 'hex',
    })
    expect(resolveOpenScadColor('#AbCd')).toEqual({
      rgba: [0xaa / 255, 0xbb / 255, 0xcc / 255, 0xdd / 255], valid: true, source: 'hex',
    })
    expect(resolveOpenScadColor('#102030')).toEqual({
      rgba: [0x10 / 255, 0x20 / 255, 0x30 / 255, 1], valid: true, source: 'hex',
    })
    expect(resolveOpenScadColor('#10203040')).toEqual({
      rgba: [0x10 / 255, 0x20 / 255, 0x30 / 255, 0x40 / 255], valid: true, source: 'hex',
    })

    const { context, warnings } = harness()
    expect(resolveOpenScadColor('#12345', undefined, context)).toEqual({
      rgba: [-1, -1, -1, -1], valid: false, source: 'invalid',
    })
    expect(resolveOpenScadColor(' red ', undefined, context).valid).toBe(false)
    expect(warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_COLOR_UNKNOWN', 'OPENSCAD_COLOR_UNKNOWN',
    ])
  })

  it('preserves OpenSCAD vector defaults, numeric conversion, alpha precedence, and range warnings', () => {
    expect(resolveOpenScadColor([]).rgba).toEqual([1, 1, 1, 1])
    expect(resolveOpenScadColor([0.1]).rgba).toEqual([0.1, 1, 1, 1])
    expect(resolveOpenScadColor([0.1, 0.2]).rgba).toEqual([0.1, 0.2, 1, 1])
    expect(resolveOpenScadColor([undefined, 'x', true, 0.4, 0.9]).rgba).toEqual([0, 0, 0, 0.4])
    expect(resolveOpenScadColor([0.1, 0.2, 0.3, 0.4], undefined).rgba)
      .toEqual([0.1, 0.2, 0.3, 0.4])
    expect(resolveOpenScadColor([0.1, 0.2, 0.3, 0.4], 0.7).rgba)
      .toEqual([0.1, 0.2, 0.3, 0.7])
    expect(resolveOpenScadColor('red', '0.5').rgba).toEqual([1, 0, 0, 1])
    expect(resolveOpenScadColor('transparent', 0.5).rgba).toEqual([0, 0, 0, 0.5])

    const { context, warnings } = harness()
    const authored = resolveOpenScadColor([-0.1, 1.2, 2, 3], undefined, context)
    expect(authored.rgba).toEqual([-0.1, 1.2, 2, 3])
    expect(warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_COLOR_CHANNEL_RANGE',
      'OPENSCAD_COLOR_CHANNEL_RANGE',
      'OPENSCAD_COLOR_CHANNEL_RANGE',
      'OPENSCAD_COLOR_ALPHA_RANGE',
    ])

    const override = harness()
    expect(resolveOpenScadColor([0.1, 0.2, 0.3, 2], 0.5, override.context).rgba)
      .toEqual([0.1, 0.2, 0.3, 0.5])
    expect(override.warnings.map(warning => warning.code)).toEqual(['OPENSCAD_COLOR_ALPHA_RANGE'])
    const alphaOnly = harness()
    expect(resolveOpenScadColor('red', 2, alphaOnly.context).rgba).toEqual([1, 0, 0, 2])
    expect(alphaOnly.warnings).toEqual([])
  })

  it('implements the radius/$fa/$fs formula and the 2021 truncation/minimum rule for explicit $fn', () => {
    expect(resolveOpenScadFragments({ radius: 10 })).toMatchObject({
      fragments: 30,
      unboundedFragments: 30,
      source: '$fa/$fs',
      effectiveFa: 12,
      effectiveFs: 2,
    })
    expect(resolveOpenScadFragments({ radius: 10, fa: 30, fs: 2 }).fragments).toBe(12)
    expect(resolveOpenScadFragments({ radius: 10, fa: 1, fs: 10 }).fragments).toBe(7)
    expect(resolveOpenScadFragments({ radius: 0 }).fragments).toBe(3)
    expect(resolveOpenScadFragments({ radius: 10, fn: 0 }).fragments).toBe(30)
    expect(resolveOpenScadFragments({ radius: 10, fn: 0.1 }).fragments).toBe(3)
    expect(resolveOpenScadFragments({ radius: 10, fn: 3 }).fragments).toBe(3)
    expect(resolveOpenScadFragments({ radius: 10, fn: 3.9 }).fragments).toBe(3)
    expect(resolveOpenScadFragments({ radius: 10, fn: 7.9 }).fragments).toBe(7)
    expect(resolveOpenScadFragments({ radius: 10, fn: 8 }).fragments).toBe(8)
    expect(resolveOpenScadFragments({ radius: 10, fn: Infinity }).fragments).toBe(3)
    expect(resolveOpenScadFragments({ radius: 10, fn: -Infinity }).fragments).toBe(3)
    expect(resolveOpenScadFragments({ radius: 10, fn: Number.NaN }).fragments).toBe(3)
    expect(resolveOpenScadFragments({ radius: 1 / 1_048_576 - 1e-12, fn: 100 })).toMatchObject({
      fragments: 3,
      source: 'geometry-epsilon',
    })
    expect(resolveOpenScadFragments({ radius: 1 / 1_048_576, fn: 100 }).fragments).toBe(100)
  })

  it('normalizes special-variable edges and reports only bounded soft warnings', () => {
    const { context, warnings } = harness()
    const automatic = resolveOpenScadFragments({ radius: 10, fn: -2, fa: 0, fs: -1 }, context)
    expect(automatic).toMatchObject({
      fragments: 256,
      unboundedFragments: 6284,
      effectiveFn: 0,
      effectiveFa: 0.01,
      effectiveFs: 0.01,
    })
    expect(warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_FN_NEGATIVE', 'OPENSCAD_FA_TOO_SMALL', 'OPENSCAD_FS_TOO_SMALL',
      'OPENSCAD_FRAGMENTS_CLAMPED',
    ])

    const preview = harness()
    const clamped = resolveOpenScadFragments({ radius: 10, fn: 1000, quality: 'preview' }, preview.context)
    expect(clamped).toMatchObject({
      fragments: 48,
      unboundedFragments: 1000,
      maximum: 48,
      reduced: true,
    })
    expect(preview.warnings).toContainEqual(expect.objectContaining({
      code: 'OPENSCAD_FRAGMENTS_CLAMPED', limit: 48,
    }))

    const undefinedSpecials = harness()
    expect(resolveOpenScadFragments({ radius: 10, fa: undefined, fs: undefined }, undefinedSpecials.context))
      .toMatchObject({ effectiveFa: 0.01, effectiveFs: 0.01 })
    expect(undefinedSpecials.warnings.map(warning => warning.code)).toEqual([
      'OPENSCAD_FA_TOO_SMALL', 'OPENSCAD_FS_TOO_SMALL', 'OPENSCAD_FRAGMENTS_CLAMPED',
    ])
  })

  it('scales full-circle fragments for partial rotate_extrude sweeps', () => {
    expect(resolveOpenScadSweepFragments({ radius: 10, sweepDegrees: 360 }).sweepFragments).toBe(30)
    expect(resolveOpenScadSweepFragments({ radius: 10, sweepDegrees: 180 }).sweepFragments).toBe(15)
    expect(resolveOpenScadSweepFragments({ radius: 10, sweepDegrees: 90 }).sweepFragments).toBe(7)
    expect(resolveOpenScadSweepFragments({ radius: 10, fn: 10.9, sweepDegrees: 180 }).sweepFragments).toBe(5)
    expect(resolveOpenScadSweepFragments({ radius: 10, sweepDegrees: -45 }).sweepFragments).toBe(3)
    expect(resolveOpenScadSweepFragments({ radius: 10, sweepDegrees: 1 }).sweepFragments).toBe(1)
    expect(resolveOpenScadSweepFragments({ radius: 10, sweepDegrees: 0 }).sweepFragments).toBe(0)
    expect(resolveOpenScadSweepFragments({ radius: 10, sweepDegrees: 720 }).sweepFragments).toBe(30)
  })

  it('resolves offset radius/delta precedence and exact join modes', () => {
    expect(resolveOpenScadOffset({})).toEqual({
      mode: 'radius', distance: 1, joinType: 'Round', chamfer: false,
    })
    expect(resolveOpenScadOffset({ r: 2, chamfer: true })).toEqual({
      mode: 'radius', distance: 2, joinType: 'Round', chamfer: false,
    })
    expect(resolveOpenScadOffset({ delta: 2 })).toEqual({
      mode: 'delta', distance: 2, joinType: 'Miter', chamfer: false,
    })
    expect(resolveOpenScadOffset({ delta: 2, chamfer: true })).toEqual({
      mode: 'delta', distance: 2, joinType: 'Square', chamfer: true,
    })
    expect(resolveOpenScadOffset({ r: undefined, delta: 2, chamfer: true })).toEqual({
      mode: 'delta', distance: 2, joinType: 'Square', chamfer: true,
    })

    const { context, warnings } = harness()
    expect(resolveOpenScadOffset({ r: 0, delta: 2 }, context)).toEqual({
      mode: 'radius', distance: 0, joinType: 'Round', chamfer: false,
    })
    expect(resolveOpenScadOffset({ r: 'not numeric', delta: 2 }, context)).toEqual({
      mode: 'delta', distance: 2, joinType: 'Miter', chamfer: false,
    })
    expect(resolveOpenScadOffset({ r: undefined, delta: 'not numeric' }, context)).toEqual({
      mode: 'radius', distance: 1, joinType: 'Round', chamfer: false,
    })
    expect(warnings).toEqual([])
  })
})
