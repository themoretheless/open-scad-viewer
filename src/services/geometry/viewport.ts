/**
 * Display geometry math. Pure-TypeScript port of math_core::viewport picking
 * helpers (unproject/client_ray/project/corner), bit-compatible with the native
 * implementation: same f64 arithmetic and the same refusal conditions, without
 * a synchronous WASM round trip on hover and projection hot paths.
 */
import { invertMatrixF64 } from './matrixInverse'
import type { Mat4, Ray3, Vec3 } from '../math3d'

function finite(values: ArrayLike<number>): boolean { return Array.from(values).every(Number.isFinite) }

function homogeneous(m: ArrayLike<number>, p: readonly number[]): number[] {
  const q = new Array<number>(4)
  for (let r = 0; r < 4; r++) q[r] = m[r * 4] * p[0] + m[r * 4 + 1] * p[1] + m[r * 4 + 2] * p[2] + m[r * 4 + 3]
  return q
}

function point(m: ArrayLike<number>, p: readonly number[]): Vec3 | null {
  const q = homogeneous(m, p)
  if (q[3] === 0 || !q.every(Number.isFinite)) return null
  const result: Vec3 = [q[0] / q[3], q[1] / q[3], q[2] / q[3]]
  return result.every(Number.isFinite) ? result : null
}

/** The supplied matrix is the inverse of projection * view (WebGPU depth 0..1). */
function unproject(inverse: ArrayLike<number>, x: number, y: number): Ray3 | null {
  const origin = point(inverse, [x, y, 0])
  const far = point(inverse, [x, y, 1])
  if (!origin || !far) return null
  const d = [far[0] - origin[0], far[1] - origin[1], far[2] - origin[2]]
  const length = Math.hypot(Math.hypot(d[0], d[1]), d[2])
  if (!Number.isFinite(length) || length === 0) return null
  return { origin, direction: d.map(v => v / length) as Vec3 }
}

/** Rect is CSS left/top/width/height; client coordinates are in the same space. */
export function clientRayInKernel(matrix: Mat4, rect: [number, number, number, number], client: [number, number]): Ray3 | null {
  if (!finite(matrix) || !finite(rect) || !finite(client)) return null
  if (rect[2] <= 0 || rect[3] <= 0) return null
  const x = (client[0] - rect[0]) / rect[2] * 2 - 1
  const y = 1 - (client[1] - rect[1]) / rect[3] * 2
  const inverse = invertMatrixF64(matrix)
  if (!inverse) return null
  return unproject(inverse, x, y)
}

export function unprojectRayInKernel(matrix: Mat4, x: number, y: number): Ray3 | null {
  if (!finite(matrix) || !finite([x, y])) return null
  return unproject(matrix, x, y)
}

export function projectPointInKernel(matrix: Mat4, point: readonly number[], size: [number, number]): [number, number] | null {
  if (!finite(matrix) || !finite(point) || !finite(size)) return null
  if (!(size[0] > 0 && size[1] > 0)) return null
  const q = homogeneous(matrix, point)
  if (q[3] <= 1e-8 || !q.every(Number.isFinite)) return null
  const result: [number, number] = [
    (q[0] / q[3] + 1) * size[0] / 2,
    (1 - q[1] / q[3]) * size[1] / 2,
  ]
  return result.every(Number.isFinite) ? result : null
}

/** Snap to the maximum-barycentric corner; exact ties keep the first corner. */
export function selectedCornerInKernel(matrix: Mat4, vertices: Vec3[], weights: Vec3): Vec3 | null {
  if (!finite(matrix) || !vertices.every(finite) || !finite(weights)) return null
  let corner = 0
  for (let i = 1; i < 3; i++) if (weights[i] > weights[corner]) corner = i
  return point(matrix, vertices[corner])
}
