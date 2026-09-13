import { describe, expect, it } from 'vitest'
import { computePanUpdate } from '../src/services/cameraGestures'
import { computeOrbitCameraFrame } from '../src/services/orbitCameraProjection'
import {
  invert, lookAt, multiply, perspective, transformPoint, unprojectRay,
  type Aabb3, type Vec3,
} from '../src/services/math3d'
import type { ProjectionMode } from '../src/services/viewportModel'
import reference from '../crates/math-core/orbit-camera-reference-v1.json'

const spinner: Aabb3 = { min: [-34, -34, 0], max: [34, 34, 10] }
const base = {
  yaw: Math.PI / 4,
  pitch: Math.atan(1 / Math.sqrt(2)),
  distance: 100,
  target: [0, 0, 5] as Vec3,
  aspect: 1.6,
  fovY: Math.PI / 4,
  projection: 'perspective' as ProjectionMode,
  bounds: spinner,
  backgroundRadius: 350,
}

it('matches 72 frozen pre-migration orbit camera frames', () => {
  expect(reference.cases).toHaveLength(72)
  for (const entry of reference.cases) {
    const actual = computeOrbitCameraFrame({ ...entry.options,
      target: entry.options.target as Vec3,
      projection: entry.options.projection as ProjectionMode,
      bounds: entry.options.bounds as Aabb3,
    })
    for (const key of ['eyeDistance', 'near', 'far'] as const)
      expect(actual[key]).toBeCloseTo(entry.expected[key], 10)
    for (let i = 0; i < 3; i++) expect(actual.eye[i]).toBeCloseTo(entry.expected.eye[i], 10)
    // f32 GPU storage permits a last-bit difference between native and JS
    // trigonometric implementations; no expected values are regenerated.
    for (let i = 0; i < 16; i++) {
      const expected = entry.expected.viewProjection[i]
      expect(Math.abs(actual.viewProjection[i] - expected)).toBeLessThanOrEqual(2e-7 * Math.max(1, Math.abs(expected)))
    }
  }
})

it('rejects invalid orbit parameters instead of publishing nonfinite matrices', () => {
  expect(() => computeOrbitCameraFrame({ ...base, aspect: 0 })).toThrow()
  expect(() => computeOrbitCameraFrame({ ...base, distance: 0 })).toThrow()
  expect(() => computeOrbitCameraFrame({ ...base, bounds: { min: [2, 2, 2], max: [1, 1, 1] } })).toThrow()
})

it('builds an empty-scene frame and keeps translated bounds inside depth planes', () => {
  const empty = computeOrbitCameraFrame({ ...base, yaw: 0, pitch: 0, target: [0, 0, 0], bounds: null })
  expect(empty.eye).toEqual([0, -100, 0])
  expect(empty.eyeDistance).toBe(100)
  expect(empty.near).toBe(5)
  expect(empty.far).toBeCloseTo(485, 12)
  for (const projection of ['perspective', 'orthographic'] as const) {
    const bounds: Aabb3 = { min: [120, -50, 20], max: [150, -10, 25] }
    const frame = computeOrbitCameraFrame({ ...base, projection, bounds, target: [130, -30, 22], distance: 0.02 })
    for (const p of corners(bounds)) {
      const depth = transformPoint(frame.viewProjection, p)[2]
      expect(depth).toBeGreaterThan(0)
      expect(depth).toBeLessThan(1)
    }
  }
})

function corners(bounds: Aabb3): Vec3[] {
  const result: Vec3[] = []
  for (const x of [bounds.min[0], bounds.max[0]]) {
    for (const y of [bounds.min[1], bounds.max[1]]) {
      for (const z of [bounds.min[2], bounds.max[2]]) result.push([x, y, z])
    }
  }
  return result
}

describe('close orbit zoom', () => {
  it.each(['orthographic', 'perspective'] as const)('keeps a whole spinner in front of the %s depth planes at every zoom and angle', projection => {
    for (const distance of [100, 30, 5, 0.01]) {
      for (const yaw of [0, Math.PI / 4, 2.7]) {
        for (const pitch of [-Math.PI / 2, -0.4, 0, Math.PI / 4, Math.PI / 2]) {
          const frame = computeOrbitCameraFrame({ ...base, distance, yaw, pitch, projection })
          for (const corner of corners(spinner)) {
            const matrix = frame.viewProjection
            const w = matrix[12] * corner[0] + matrix[13] * corner[1] + matrix[14] * corner[2] + matrix[15]
            expect(w).toBeGreaterThan(0)
            const depth = transformPoint(matrix, corner)[2]
            expect(depth).toBeGreaterThan(0)
            expect(depth).toBeLessThan(1)
          }
        }
      }
    }
  })

  it.each(['orthographic', 'perspective'] as const)('continues to magnify in %s after the eye stops at the model', projection => {
    const wide = computeOrbitCameraFrame({ ...base, distance: 2, projection })
    const close = computeOrbitCameraFrame({ ...base, distance: 1, projection })
    expect(close.eyeDistance).toBe(wide.eyeDistance)
    expect(close.eye).toEqual(wide.eye)
    const point: Vec3 = [3, 3, 5]
    const a = transformPoint(wide.viewProjection, point)
    const b = transformPoint(close.viewProjection, point)
    expect(b[0]).toBeCloseTo(a[0] * 2, 5)
    expect(b[1]).toBeCloseTo(a[1] * 2, 5)
  })

  it('retains the original perspective projection at normal viewing distances', () => {
    const frame = computeOrbitCameraFrame(base)
    expect(frame.eyeDistance).toBe(base.distance)
    const expected = multiply(
      perspective(base.fovY, base.aspect, frame.near, frame.far),
      lookAt(frame.eye, base.target, [0, 0, 1]),
    )
    expect(Array.from(frame.viewProjection)).toEqual(Array.from(expected))
  })

  it.each(['orthographic', 'perspective'] as const)('preserves pan scale and picking rays in %s at close zoom', projection => {
    const state = { yaw: base.yaw, pitch: base.pitch, dist: 0.2, tx: 0, ty: 0, tz: 5 }
    const moved = computePanUpdate(state, 30, -20, 800)
    const target: Vec3 = [moved.tx, moved.ty, moved.tz]
    const frame = computeOrbitCameraFrame({ ...base, distance: state.dist, target, projection })
    const projected = transformPoint(frame.viewProjection, base.target)
    expect(projected[0]).toBeCloseTo(2 * 30 / (800 * base.aspect), 4)
    expect(projected[1]).toBeCloseTo(2 * 20 / 800, 4)

    const ray = unprojectRay(invert(frame.viewProjection), projected[0], projected[1])
    expect(ray).not.toBeNull()
    const delta = base.target.map((value, axis) => value - ray!.origin[axis])
    const along = delta.reduce((sum, value, axis) => sum + value * ray!.direction[axis], 0)
    const error = Math.hypot(...delta.map((value, axis) => value - along * ray!.direction[axis]))
    expect(along).toBeGreaterThan(0)
    expect(error).toBeLessThan(0.001)
  })

  it.each(['orthographic', 'perspective'] as const)('protects translated, panned and very thin models in %s', projection => {
    const bounds: Aabb3 = { min: [960, -240, 14.999], max: [1040, -160, 15.001] }
    const frame = computeOrbitCameraFrame({
      ...base, projection, bounds, distance: 0.01, target: [990, -190, 12], pitch: Math.PI / 2,
    })
    for (const corner of corners(bounds)) {
      const depth = transformPoint(frame.viewProjection, corner)[2]
      expect(depth).toBeGreaterThan(0)
      expect(depth).toBeLessThan(1)
    }
  })
})
