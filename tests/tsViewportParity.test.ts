import { describe, expect, it } from 'vitest'
import { callGeometryRust } from '../src/services/geometry/kernel'
import { invertMatrixF64 } from '../src/services/geometry/matrixInverse'
import { invert, identity, multiply, perspective, orthographic, lookAt, translate, type Mat4, type Vec3, type Aabb3 } from '../src/services/math3d'
import { computeOrbitCameraFrame } from '../src/services/orbitCameraProjection'
import { clientRayInKernel, projectPointInKernel } from '../src/services/geometry/viewport'

/** Relative-or-absolute closeness within the 1e-12 audit tolerance. */
const close = (actual: number, expected: number) =>
  expect(Math.abs(actual - expected)).toBeLessThanOrEqual(1e-12 * Math.max(1, Math.abs(expected)))

/**
 * f64 outputs of trigonometric kernels: V8 and the wasm libm may differ by a
 * few ulps of the operand magnitude, so a fixed 1e-12 absolute bound is not
 * attainable for near-zero components of large-magnitude expressions. Allow a
 * handful of f64 ulps at the dominating scale.
 */
const closeTrig = (actual: number, expected: number, scale: number) =>
  expect(Math.abs(actual - expected)).toBeLessThanOrEqual(8 * Number.EPSILON * Math.max(1, Math.abs(expected), scale))

/** f32 GPU storage: values are identical unless a rounding boundary flips. */
const closeF32 = (actual: number, expected: number) =>
  expect(Math.abs(actual - expected)).toBeLessThanOrEqual(2 * 2 ** -23 * Math.max(1, Math.abs(expected)))

function wasmInverse(matrix: Mat4): number[] | null {
  try {
    return callGeometryRust<number[]>('viewport', { action: 'inverse', matrix: Array.from(matrix) })
  } catch {
    return null
  }
}

function testMatrices(): Mat4[] {
  const matrices: Mat4[] = [identity()]
  let seed = 42
  const random = () => (seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff - 0.5
  for (let i = 0; i < 24; i++) {
    const m = new Float32Array(16)
    for (let k = 0; k < 16; k++) m[k] = random() * 8
    // Keep most random matrices invertible but not trivially conditioned.
    m[0] += 4; m[5] += 4; m[10] += 4; m[15] += 4
    matrices.push(m)
  }
  matrices.push(translate(identity(), [3, -2, 7]))
  matrices.push(perspective(1.1, 1.7, 0.1, 1000))
  matrices.push(orthographic(-7, 7, -4, 4, 0.1, 1000))
  matrices.push(multiply(perspective(0.9, 1.3, 0.5, 200), lookAt([7, -5, 9], [1, 2, -1], [0, 0, 1])))
  const small = identity()
  small[0] = small[5] = small[10] = 1e-4
  matrices.push(small)
  return matrices
}

describe('pure-TS viewport math parity with the WASM kernel', () => {
  it('invert matches the native inverse within 1e-12 and refuses the same matrices', () => {
    for (const matrix of testMatrices()) {
      const native = wasmInverse(matrix)
      if (native === null) {
        expect(invertMatrixF64(matrix)).toBeNull()
        expect(() => invert(matrix)).toThrow('Singular')
        continue
      }
      const ts = invert(matrix)
      for (let i = 0; i < 16; i++) close(ts[i], native[i])
    }
    // Both paths refuse a zero matrix.
    expect(wasmInverse(new Float32Array(16))).toBeNull()
    expect(() => invert(new Float32Array(16))).toThrow('Singular')
  })

  it('computeOrbitCameraFrame matches the native orbit frame within 1e-12', () => {
    const bounds: Aabb3 = { min: [-34, -34, 0], max: [34, 34, 10] }
    for (const projection of ['perspective', 'orthographic'] as const) {
      for (const yaw of [0, Math.PI / 4, 2.7]) {
        for (const pitch of [-Math.PI / 2, -0.4, 0, Math.PI / 4, Math.PI / 2]) {
          for (const distance of [100, 30, 5, 0.01]) {
            const options = {
              yaw, pitch, distance,
              target: [0, 0, 5] as Vec3,
              aspect: 1.6, fovY: Math.PI / 4, projection,
              bounds, backgroundRadius: 350,
            }
            const native = callGeometryRust<{
              eye: Vec3; viewProjection: number[]; eyeDistance: number; near: number; far: number
            }>('viewport', { ...options, action: 'orbit' })
            const ts = computeOrbitCameraFrame(options)
            for (let i = 0; i < 3; i++) closeTrig(ts.eye[i], native.eye[i], ts.eyeDistance)
            for (let i = 0; i < 16; i++) closeF32(ts.viewProjection[i], native.viewProjection[i])
            closeTrig(ts.eyeDistance, native.eyeDistance, ts.eyeDistance)
            closeTrig(ts.near, native.near, ts.far)
            closeTrig(ts.far, native.far, ts.far)
          }
        }
      }
    }
    // Empty-scene frame too.
    const options = {
      yaw: 0, pitch: 0, distance: 100, target: [0, 0, 0] as Vec3,
      aspect: 1.6, fovY: Math.PI / 4, projection: 'perspective' as const,
      bounds: null, backgroundRadius: 350,
    }
    const native = callGeometryRust<{ eye: Vec3; viewProjection: number[] }>('viewport', { ...options, action: 'orbit' })
    const ts = computeOrbitCameraFrame(options)
    for (let i = 0; i < 16; i++) closeF32(ts.viewProjection[i], native.viewProjection[i])
  })

  it('client rays and projections match the native implementations within 1e-12', () => {
    const view = lookAt([7, -5, 9], [1, 2, -1], [0, 0, 1])
    for (const projection of [perspective(1.1, 1.7, 0.1, 1000), orthographic(-7, 7, -4, 4, 0.1, 1000)]) {
      const matrix = multiply(projection, view)
      for (const [cx, cy] of [[17, 31], [442, 181], [867, 531]]) {
        const rect: [number, number, number, number] = [17, 31, 850, 500]
        const native = callGeometryRust<{ origin: Vec3; direction: Vec3 } | null>(
          'viewport', { action: 'client_ray', matrix: Array.from(matrix), rect, client: [cx, cy] })
        const ts = clientRayInKernel(matrix, rect, [cx, cy])
        expect(ts === null).toBe(native === null)
        if (ts && native) {
          for (let i = 0; i < 3; i++) {
            close(ts.origin[i], native.origin[i])
            close(ts.direction[i], native.direction[i])
          }
        }
      }
      for (const point of [[0, 0, -5], [3, -2, -40]]) {
        const size: [number, number] = [800, 600]
        const native = callGeometryRust<[number, number] | null>(
          'viewport', { action: 'project', matrix: Array.from(matrix), point, size })
        const ts = projectPointInKernel(matrix, point, size)
        expect(ts === null).toBe(native === null)
        if (ts && native) {
          close(ts[0], native[0])
          close(ts[1], native[1])
        }
      }
    }
  })
})
