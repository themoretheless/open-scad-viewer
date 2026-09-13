/** Display geometry transport. Calculations execute in math_core::viewport. */
import { callGeometryRust } from './kernel'
import type { Mat4, Ray3, Vec3 } from '../math3d'

function finite(values: ArrayLike<number>): boolean { return Array.from(values).every(Number.isFinite) }
export function clientRayInKernel(matrix: Mat4, rect: [number, number, number, number], client: [number, number]): Ray3 | null {
  if (!finite(matrix) || !finite(rect) || !finite(client)) return null
  return callGeometryRust('viewport', { action: 'client_ray', matrix: Array.from(matrix), rect, client })
}
export function unprojectRayInKernel(matrix: Mat4, x: number, y: number): Ray3 | null {
  if (!finite(matrix) || !finite([x, y])) return null
  return callGeometryRust('viewport', { action: 'unproject', matrix: Array.from(matrix), x, y })
}
export function projectPointInKernel(matrix: Mat4, point: readonly number[], size: [number, number]): [number, number] | null {
  if (!finite(matrix) || !finite(point) || !finite(size)) return null
  return callGeometryRust('viewport', { action: 'project', matrix: Array.from(matrix), point: Array.from(point), size })
}
export function selectedCornerInKernel(matrix: Mat4, vertices: Vec3[], weights: Vec3): Vec3 | null {
  if (!finite(matrix) || !vertices.every(finite) || !finite(weights)) return null
  return callGeometryRust('viewport', { action: 'corner', matrix: Array.from(matrix), vertices, weights })
}
