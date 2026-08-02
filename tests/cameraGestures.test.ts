import { describe, expect, it } from 'vitest'
import {
  computeOrbitUpdate,
  computePanUpdate,
  computeWheelDistance,
} from '../src/services/cameraGestures'

const initial = { yaw: 0, pitch: 0, dist: 10, tx: 0, ty: 0, tz: 0 }

describe('camera gesture math', () => {
  it('wraps orbit yaw and clamps pitch without mutating input', () => {
    const state = { ...initial, yaw: Math.PI - 0.001 }
    const next = computeOrbitUpdate(state, -10, 1000)
    expect(next.yaw).toBeGreaterThanOrEqual(-Math.PI)
    expect(next.yaw).toBeLessThanOrEqual(Math.PI)
    expect(next.pitch).toBeLessThan(Math.PI / 2)
    expect(state).toEqual({ ...initial, yaw: Math.PI - 0.001 })
  })

  it('uses the orbit basis for deterministic world-space pan', () => {
    const next = computePanUpdate(initial, 10, 5, 100)
    expect(next.tx).toBeLessThan(0)
    expect(next.ty).toBeCloseTo(0)
    expect(next.tz).toBeGreaterThan(0)
  })

  it('clamps wheel input and distance bounds', () => {
    expect(computeWheelDistance(10, 5000)).toBeCloseTo(10 * Math.E)
    expect(computeWheelDistance(10, -5000)).toBeCloseTo(10 / Math.E)
    expect(computeWheelDistance(1e12, 1000)).toBe(1e12)
  })
})
