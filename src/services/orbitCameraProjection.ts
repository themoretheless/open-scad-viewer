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

const SCRATCH_PROJECTION = new Float32Array(16)
const SCRATCH_VIEW = new Float32Array(16)

/** Row-major view matrix looking from eye to target with a fixed Z-up world. */
function writeOrbitViewMatrix(eye: Vec3, target: Vec3, out: Float32Array): void {
  let zx = eye[0] - target[0]
  let zy = eye[1] - target[1]
  let zz = eye[2] - target[2]
  let norm = length3(zx, zy, zz)
  if (norm < 1e-10) {
    zx = 0; zy = 0; zz = 1
    norm = 1
  }
  zx /= norm; zy /= norm; zz /= norm

  // cross3([0, 0, 1], z) = [-zy, zx, 0]
  let xx = 0 * zz - 1 * zy
  let xy = 1 * zx - 0 * zz
  let xz = 0 * zy - 0 * zx
  norm = length3(xx, xy, xz)
  if (norm < 1e-10) {
    const highZ = Math.abs(zz) > 0.9
    const ux = 0, uy = highZ ? 1 : 0, uz = highZ ? 0 : 1
    xx = uy * zz - uz * zy
    xy = uz * zx - ux * zz
    xz = ux * zy - uy * zx
    norm = length3(xx, xy, xz)
  }
  xx /= norm; xy /= norm; xz /= norm

  const yx = zy * xz - zz * xy
  const yy = zz * xx - zx * xz
  const yz = zx * xy - zy * xx

  out[0] = xx; out[1] = xy; out[2] = xz; out[3] = -(xx * eye[0] + xy * eye[1] + xz * eye[2])
  out[4] = yx; out[5] = yy; out[6] = yz; out[7] = -(yx * eye[0] + yy * eye[1] + yz * eye[2])
  out[8] = zx; out[9] = zy; out[10] = zz; out[11] = -(zx * eye[0] + zy * eye[1] + zz * eye[2])
  out[12] = 0; out[13] = 0; out[14] = 0; out[15] = 1
}

export function computeOrbitCameraFrame(o: OrbitProjectionOptions) {
  const invalid = (): never => { throw new Error('Invalid or unrepresentable orbit camera') }
  if (o.projection !== 'perspective' && o.projection !== 'orthographic') invalid()
  if (
    !Number.isFinite(o.yaw) ||
    !Number.isFinite(o.pitch) ||
    !Number.isFinite(o.distance) ||
    !Number.isFinite(o.aspect) ||
    !Number.isFinite(o.fovY) ||
    !Number.isFinite(o.backgroundRadius) ||
    !Number.isFinite(o.target[0]) ||
    !Number.isFinite(o.target[1]) ||
    !Number.isFinite(o.target[2]) ||
    o.distance <= 0 ||
    o.aspect <= 0 ||
    o.fovY <= 0 ||
    o.fovY >= Math.PI ||
    o.backgroundRadius < 0
  ) {
    invalid()
  }

  const cp = Math.cos(o.pitch)
  const bx = cp * Math.sin(o.yaw)
  const by = -cp * Math.cos(o.yaw)
  const bz = Math.sin(o.pitch)
  let frontDepth = 0
  let eyeDistance = o.distance
  let extent = o.backgroundRadius
  if (o.bounds) {
    const { min, max } = o.bounds
    for (let i = 0; i < 3; i++) {
      if (!(Number.isFinite(min[i]) && Number.isFinite(max[i]) && min[i] <= max[i])) invalid()
    }
    const cx = min[0] * 0.5 + max[0] * 0.5
    const cy = min[1] * 0.5 + max[1] * 0.5
    const cz = min[2] * 0.5 + max[2] * 0.5
    const hx = max[0] * 0.5 - min[0] * 0.5
    const hy = max[1] * 0.5 - min[1] * 0.5
    const hz = max[2] * 0.5 - min[2] * 0.5
    const ox = cx - o.target[0]
    const oy = cy - o.target[1]
    const oz = cz - o.target[2]
    frontDepth =
      (bx * ox + by * oy + bz * oz) +
      (Math.abs(bx) * hx + Math.abs(by) * hy + Math.abs(bz) * hz)
    const radius = length3(hx, hy, hz)
    eyeDistance = Math.max(eyeDistance, frontDepth + Math.max(0.01, radius * 0.05))
    extent = Math.max(extent, length3(ox, oy, oz) + radius)
  }
  const eye: Vec3 = [
    o.target[0] + eyeDistance * bx,
    o.target[1] + eyeDistance * by,
    o.target[2] + eyeDistance * bz,
  ]
  const near = Math.max(0.001, (eyeDistance - frontDepth) * 0.05)
  const far = Math.max(near + 1, eyeDistance + extent * 1.1)
  const halfHeight = o.distance * Math.tan(o.fovY / 2)
  const nf = 1 / (near - far)
  SCRATCH_PROJECTION.fill(0)
  if (o.projection === 'perspective') {
    const f = 1 / Math.tan(Math.atan(halfHeight / eyeDistance))
    SCRATCH_PROJECTION[0] = f / o.aspect
    SCRATCH_PROJECTION[5] = f
    SCRATCH_PROJECTION[10] = far * nf
    SCRATCH_PROJECTION[11] = far * near * nf
    SCRATCH_PROJECTION[14] = -1
  } else {
    SCRATCH_PROJECTION[0] = 2 / (2 * halfHeight * o.aspect)
    SCRATCH_PROJECTION[5] = 2 / (2 * halfHeight)
    SCRATCH_PROJECTION[10] = nf
    SCRATCH_PROJECTION[11] = near * nf
    SCRATCH_PROJECTION[15] = 1
  }
  writeOrbitViewMatrix(eye, o.target, SCRATCH_VIEW)
  const viewProjection = new Float32Array(16)
  for (let r = 0; r < 4; r++) {
    const r4 = r * 4
    const p0 = SCRATCH_PROJECTION[r4]
    const p1 = SCRATCH_PROJECTION[r4 + 1]
    const p2 = SCRATCH_PROJECTION[r4 + 2]
    const p3 = SCRATCH_PROJECTION[r4 + 3]
    for (let c = 0; c < 4; c++) {
      const sum =
        p0 * SCRATCH_VIEW[c] +
        p1 * SCRATCH_VIEW[4 + c] +
        p2 * SCRATCH_VIEW[8 + c] +
        p3 * SCRATCH_VIEW[12 + c]
      const v = fround(sum)
      if (!Number.isFinite(v)) invalid()
      viewProjection[r4 + c] = v
    }
  }
  if (
    !Number.isFinite(eye[0]) ||
    !Number.isFinite(eye[1]) ||
    !Number.isFinite(eye[2]) ||
    !Number.isFinite(eyeDistance) ||
    !Number.isFinite(near) ||
    !Number.isFinite(far)
  ) {
    invalid()
  }

  return { eye, viewProjection, eyeDistance, near, far }
}
