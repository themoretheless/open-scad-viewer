import { describe, expect, it } from 'vitest'
import {
  computeOrbitUpdate,
  computePanUpdate,
  computeWheelDistance,
  computePinchUpdate,
  clampGestureDistance,
} from '../src/services/cameraGestures'
import reference from '../crates/math-core/camera-gestures-reference-v1.json'

const initial = { yaw: 0, pitch: 0, dist: 10, tx: 0, ty: 0, tz: 0 }

it('matches 81 frozen pre-migration orbit, pan and pinch states', () => {
  expect(reference.cases).toHaveLength(81)
  for (const entry of reference.cases) {
    const result = entry.action === 'orbit'
      ? computeOrbitUpdate(...entry.args as unknown as Parameters<typeof computeOrbitUpdate>)
      : entry.action === 'pan'
        ? computePanUpdate(...entry.args as unknown as Parameters<typeof computePanUpdate>)
        : computePinchUpdate(...entry.args as unknown as Parameters<typeof computePinchUpdate>)
    for (const key of ['yaw', 'pitch', 'dist', 'tx', 'ty', 'tz'] as const) {
      expect(Math.abs(result[key] - entry.expected[key])).toBeLessThanOrEqual(1e-12 * Math.max(1, Math.abs(entry.expected[key])))
    }
  }
})

it('preserves explicit scalar fallbacks across the finite-only WASM transport', () => {
  for (const value of [NaN, Infinity, -Infinity]) {
    expect(clampGestureDistance(value)).toBe(50)
    expect(computeWheelDistance(value, 1)).toBe(50)
  }
  expect(computeWheelDistance(10, NaN)).toBe(50)
  expect(computeWheelDistance(10, Infinity)).toBeCloseTo(10 * Math.E, 12)
  expect(computeWheelDistance(10, -Infinity)).toBeCloseTo(10 / Math.E, 12)
  expect(clampGestureDistance(-10)).toBe(0.01)
})

it('pans a collapsed pinch without zoom and leaves rejected input unchanged', () => {
  const p = { x: 0, y: 0 }, q = { x: 10, y: 10 }
  const result = computePinchUpdate(p, p, q, q, initial, 100)
  expect(result.dist).toBe(initial.dist)
  expect(result.tx).toBeLessThan(initial.tx)
  expect(result.tz).toBeGreaterThan(initial.tz)
  const input = { ...initial }
  expect(() => computeOrbitUpdate(input, NaN, 0)).toThrow()
  expect(input).toEqual(initial)
})

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
