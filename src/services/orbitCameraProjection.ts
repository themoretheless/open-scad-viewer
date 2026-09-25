import type { Aabb3, Vec3 } from './math3d'
import type { ProjectionMode } from './viewportModel'

interface OrbitProjectionOptions {
  yaw: number
  pitch: number
  /** Apparent zoom distance at the orbit target, also used by pan gestures. */
  distance: number
  target: Vec3
  aspect: number
  fovY: number
  projection: ProjectionMode
  bounds: Aabb3 | null
  backgroundRadius: number
}

/**
 * Pure-TypeScript port of math_core::orbit_camera::frame. The computation runs
 * in f64 while matrices are stored through the same f32 GPU transport round
 * trip as the native implementation, so results match the WASM path bit-for-bit
 * without a synchronous WASM call per frame. Invalid parameters throw exactly
 * like the native error path.
 */

const fround = Math.fround
/** Native `length` is f64::hypot applied pairwise; mirror the association order. */
const length3 = (x: number, y: number, z: number) => Math.hypot(Math.hypot(x, y), z)
const dot3 = (a: number[], b: number[]) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
const cross3 = (a: number[], b: number[]) => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
]

/** Row-major view matrix looking from eye to target with a fixed Z-up world. */
function orbitViewMatrix(eye: number[], target: Vec3): number[] {
  let z = [eye[0] - target[0], eye[1] - target[1], eye[2] - target[2]]
  let norm = length3(z[0], z[1], z[2])
  if (norm < 1e-10) {
    z = [0, 0, 1]
    norm = 1
  }
  z = z.map(v => v / norm)
  let x = cross3([0, 0, 1], z)
  norm = length3(x[0], x[1], x[2])
  if (norm < 1e-10) {
    x = cross3(Math.abs(z[2]) > 0.9 ? [0, 1, 0] : [0, 0, 1], z)
    norm = length3(x[0], x[1], x[2])
  }
  x = x.map(v => v / norm)
  const y = cross3(z, x)
  return [
    x[0], x[1], x[2], -dot3(x, eye),
    y[0], y[1], y[2], -dot3(y, eye),
    z[0], z[1], z[2], -dot3(z, eye),
    0, 0, 0, 1,
  ].map(fround)
}

export function computeOrbitCameraFrame(o: OrbitProjectionOptions) {
  const invalid = (): never => { throw new Error('Invalid or unrepresentable orbit camera') }
  if (o.projection !== 'perspective' && o.projection !== 'orthographic') invalid()
  if (![o.yaw, o.pitch, o.distance, o.aspect, o.fovY, o.backgroundRadius].every(Number.isFinite)
    || !o.target.every(Number.isFinite)
    || o.distance <= 0
    || o.aspect <= 0
    || o.fovY <= 0
    || o.fovY >= Math.PI
    || o.backgroundRadius < 0) invalid()

  const cp = Math.cos(o.pitch)
  const backward = [cp * Math.sin(o.yaw), -cp * Math.cos(o.yaw), Math.sin(o.pitch)]
  let frontDepth = 0
  let eyeDistance = o.distance
  let extent = o.backgroundRadius
  if (o.bounds) {
    const { min, max } = o.bounds
    for (let i = 0; i < 3; i++) {
      if (!(Number.isFinite(min[i]) && Number.isFinite(max[i]) && min[i] <= max[i])) invalid()
    }
    const center = min.map((v, i) => v * 0.5 + max[i] * 0.5)
    const half = max.map((v, i) => v * 0.5 - min[i] * 0.5)
    const offset = center.map((v, i) => v - o.target[i])
    frontDepth = dot3(backward, offset) + dot3(backward.map(Math.abs), half)
    const radius = length3(half[0], half[1], half[2])
    eyeDistance = Math.max(eyeDistance, frontDepth + Math.max(0.01, radius * 0.05))
    extent = Math.max(extent, length3(offset[0], offset[1], offset[2]) + radius)
  }
  const eye = o.target.map((v, i) => v + eyeDistance * backward[i])
  const near = Math.max(0.001, (eyeDistance - frontDepth) * 0.05)
  const far = Math.max(near + 1, eyeDistance + extent * 1.1)
  const halfHeight = o.distance * Math.tan(o.fovY / 2)
  const nf = 1 / (near - far)
  const projection = new Array<number>(16).fill(0)
  if (o.projection === 'perspective') {
    const f = 1 / Math.tan(Math.atan(halfHeight / eyeDistance))
    projection[0] = f / o.aspect
    projection[5] = f
    projection[10] = far * nf
    projection[11] = far * near * nf
    projection[14] = -1
  } else {
    projection[0] = 2 / (2 * halfHeight * o.aspect)
    projection[5] = 2 / (2 * halfHeight)
    projection[10] = nf
    projection[11] = near * nf
    projection[15] = 1
  }
  const storedProjection = projection.map(fround)
  const view = orbitViewMatrix(eye, o.target)
  const matrix = new Array<number>(16)
  for (let r = 0; r < 4; r++) {
    for (let c = 0; c < 4; c++) {
      let sum = 0
      for (let k = 0; k < 4; k++) sum += storedProjection[r * 4 + k] * view[k * 4 + c]
      matrix[r * 4 + c] = fround(sum)
    }
  }
  if (!matrix.every(Number.isFinite)
    || !eye.every(Number.isFinite)
    || ![eyeDistance, near, far].every(Number.isFinite)) invalid()

  return { eye: eye as Vec3, viewProjection: new Float32Array(matrix), eyeDistance, near, far }
}
