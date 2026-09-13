import { expect, it } from 'vitest'
import { identity, lookAt, multiply, orthographic, perspective } from '../src/services/math3d'
import { clientRayInKernel, projectPointInKernel, selectedCornerInKernel } from '../src/services/geometry/viewport'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

it('constructs perspective and orthographic center rays with WebGPU near depth', () => {
  for (const matrix of [perspective(Math.PI / 2, 2, 1, 10), orthographic(-2, 2, -1, 1, 1, 10)]) {
    const ray = clientRayInKernel(matrix, [10, 20, 400, 200], [210, 120])!
    expect(ray.origin[0]).toBeCloseTo(0, 12)
    expect(ray.origin[1]).toBeCloseTo(0, 12)
    expect(ray.origin[2]).toBeCloseTo(-1, 6)
    expect(ray.direction).toEqual([0, 0, -1])
  }
})

it('places ray origins on the near plane and keeps rays under their CSS pixel', () => {
  const view = lookAt([7, -5, 9], [1, 2, -1], [0, 0, 1])
  for (const projection of [perspective(1.1, 1.7, 0.1, 1000), orthographic(-7, 7, -4, 4, 0.1, 1000)]) {
    const matrix = multiply(projection, view)
    for (const x of [0, 0.2, 0.5, 0.9, 1]) for (const y of [0, 0.3, 0.5, 1]) {
      const ray = clientRayInKernel(matrix, [17, 31, 850, 500], [17 + x * 850, 31 + y * 500])!
      expect(Math.hypot(...ray.direction)).toBeCloseTo(1, 12)
      for (const t of [0, 2, 20]) {
        const p = ray.origin.map((v, a) => v + t * ray.direction[a])
        // Independent forward multiplication of the supplied matrix, not a
        // roundtrip through the native project implementation.
        const q = [0, 1, 2, 3].map(r => matrix[r*4]*p[0]+matrix[r*4+1]*p[1]+matrix[r*4+2]*p[2]+matrix[r*4+3])
        expect(q[0] / q[3]).toBeCloseTo(x * 2 - 1, 9)
        expect(q[1] / q[3]).toBeCloseTo(1 - y * 2, 9)
        if (t === 0) expect(q[2] / q[3]).toBeCloseTo(0, 9)
      }
    }
  }
})

it('refuses invalid viewports and singular matrices without inventing a ray', () => {
  expect(clientRayInKernel(new Float32Array(16), [0, 0, 100, 100], [50, 50])).toBeNull()
  expect(clientRayInKernel(identity(), [0, 0, 0, 100], [0, 0])).toBeNull()
  expect(clientRayInKernel(identity(), [0, 0, 100, 100], [NaN, 0])).toBeNull()
  expect(projectPointInKernel(perspective(1, 1, 1, 10), [0, 0, 1], [100, 100])).toBeNull()
})

it('projects CSS coordinates and snaps transformed corners with stable ties', () => {
  const matrix = identity()
  matrix[0] = 2; matrix[3] = 5
  const vertices: [number, number, number][] = [[1, 0, 0], [2, 0, 0], [3, 0, 0]]
  expect(selectedCornerInKernel(matrix, vertices, [0.5, 0.5, 0])).toEqual([7, 0, 0])
  expect(selectedCornerInKernel(matrix, vertices, [0.1, 0.2, 0.7])).toEqual([11, 0, 0])
  expect(projectPointInKernel(identity(), [0.5, 0.5, 0], [200, 100])).toEqual([150, 25])
})

it('routes public renderer ray/projection through native viewport calculations', () => {
  const renderer = new WebGPURenderer()
  const internal = renderer as unknown as { canvas: HTMLCanvasElement | null; cameraState(): { viewProjection: Float32Array } }
  internal.canvas = { clientWidth: 200, clientHeight: 100,
    getBoundingClientRect: () => ({ left: 10, top: 20, width: 200, height: 100 }),
  } as unknown as HTMLCanvasElement
  internal.cameraState = () => ({ viewProjection: identity() })
  try {
    expect(renderer.worldRay(160, 45)).toEqual({ origin: [0.5, 0.5, 0], direction: [0, 0, 1] })
    expect(renderer.projectWorldPoint([0.5, 0.5, 0])).toEqual([150, 25])
    internal.cameraState = () => ({ viewProjection: new Float32Array(16) })
    expect(renderer.worldRay(160, 45)).toBeNull()
  } finally { internal.canvas = null; renderer.destroy() }
})
