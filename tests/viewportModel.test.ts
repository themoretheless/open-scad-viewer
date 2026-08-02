import { describe, expect, it } from 'vitest'
import {
  projectAxesToScreen,
  standardViewForCamera,
  standardViewOrientation,
  type StandardView,
} from '../src/services/viewportModel'

describe('viewport model', () => {
  it('round-trips every canonical standard view and treats wrapped yaw as equivalent', () => {
    const views: StandardView[] = ['iso', 'front', 'back', 'left', 'right', 'top', 'bottom']
    for (const view of views) {
      const [yaw, pitch] = standardViewOrientation(view)
      expect(standardViewForCamera({ yaw, pitch })).toBe(view)
    }
    expect(standardViewForCamera({ yaw: -Math.PI, pitch: 0 })).toBe('back')
  })

  it('keeps pole projection finite and canonical', () => {
    expect(projectAxesToScreen(1.7, Math.PI / 2)).toEqual({
      x: { x: 1, y: 0, depth: 0 },
      y: { x: 0, y: 1, depth: 0 },
      z: { x: 0, y: 0, depth: 1 },
    })
  })
})
