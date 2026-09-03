import { describe, expect, it } from 'vitest'
import {
  createOpenScadStableRuntimeVariables,
  OPENSCAD_2021_DEFAULT_VIEW,
} from '../src/services/openScadStableRuntime'

describe('OpenSCAD 2021.01 host runtime variables', () => {
  it('uses the pinned 2021 camera, tessellation, animation and render defaults', () => {
    expect(Object.fromEntries(createOpenScadStableRuntimeVariables({ quality: 'full' }))).toEqual({
      $fn: 0,
      $fa: 12,
      $fs: 2,
      $t: 0,
      $preview: false,
      $vpt: [0, 0, 0],
      $vpr: [55, 0, 25],
      $vpd: 140,
      $vpf: 22.5,
    })
    expect(OPENSCAD_2021_DEFAULT_VIEW).toEqual({
      translation: [0, 0, 0],
      rotation: [55, 0, 25],
      distance: 140,
      fieldOfView: 22.5,
    })
  })

  it('derives preview and accepts only the bounded animation interval', () => {
    const preview = createOpenScadStableRuntimeVariables({
      quality: 'preview',
      animationTime: 0.375,
    })
    expect(preview.get('$t')).toBe(0.375)
    expect(preview.get('$preview')).toBe(true)
    for (const animationTime of [-0.001, 1.001, Infinity, Number.NaN]) {
      expect(() => createOpenScadStableRuntimeVariables({ quality: 'full', animationTime }))
        .toThrow(RangeError)
    }
  })

  it('returns independent maps and vectors for dynamic-scope mutation', () => {
    const first = createOpenScadStableRuntimeVariables({ quality: 'full' })
    const second = createOpenScadStableRuntimeVariables({ quality: 'full' })
    expect(first).not.toBe(second)
    expect(first.get('$vpt')).not.toBe(second.get('$vpt'))
  })
})
