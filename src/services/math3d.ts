/** Row-major matrix transport and remaining vector helpers. */
import { callGeometryRust } from './geometry/kernel'

export type Mat4 = Float32Array
export type Vec3 = [number, number, number]

export interface Ray3 {
  origin: Vec3
  direction: Vec3
}

export interface Aabb3 {
  min: Vec3
  max: Vec3
}

export function identity(): Mat4 {
  const m = new Float32Array(16)
  m[0] = m[5] = m[10] = m[15] = 1
  return m
}

export function multiply(a: Mat4, b: Mat4): Mat4 {
  return new Float32Array(callGeometryRust<number[]>('viewport', {
    action: 'multiply', matrix: Array.from(a), other: Array.from(b),
  }))
}

export function translate(m: Mat4, v: Vec3): Mat4 {
  const t = identity()
  t[3] = v[0]; t[7] = v[1]; t[11] = v[2]
  return multiply(t, m)
}

export function rotateX(m: Mat4, rad: number): Mat4 {
  const c = Math.cos(rad), s = Math.sin(rad), r = identity()
  r[5] = c; r[6] = -s; r[9] = s; r[10] = c
  return multiply(r, m)
}

export function rotateY(m: Mat4, rad: number): Mat4 {
  const c = Math.cos(rad), s = Math.sin(rad), r = identity()
  r[0] = c; r[2] = s; r[8] = -s; r[10] = c
  return multiply(r, m)
}

export function rotateZ(m: Mat4, rad: number): Mat4 {
  const c = Math.cos(rad), s = Math.sin(rad), r = identity()
  r[0] = c; r[1] = -s; r[4] = s; r[5] = c
  return multiply(r, m)
}

export function scale(m: Mat4, v: Vec3): Mat4 {
  const s = identity()
  s[0] = v[0]; s[5] = v[1]; s[10] = v[2]
  return multiply(s, m)
}

export function perspective(fov: number, aspect: number, near: number, far: number): Mat4 {
  const f = 1 / Math.tan(fov / 2), nf = 1 / (near - far)
  const m = new Float32Array(16)
  m[0] = f / aspect; m[5] = f
  // WebGPU's normalized device coordinate depth range is [0, 1].
  // The matrices in this module are row-major and use a right-handed camera
  // looking down -Z, hence z=-near maps to 0 and z=-far maps to 1.
  m[10] = far * nf; m[11] = far * near * nf
  m[14] = -1
  return m
}

/** Right-handed WebGPU orthographic projection (depth range [0, 1]). */
export function orthographic(
  left: number,
  right: number,
  bottom: number,
  top: number,
  near: number,
  far: number,
): Mat4 {
  const lr = 1 / (right - left)
  const bt = 1 / (top - bottom)
  const nf = 1 / (near - far)
  const m = new Float32Array(16)
  m[0] = 2 * lr
  m[3] = -(right + left) * lr
  m[5] = 2 * bt
  m[7] = -(top + bottom) * bt
  m[10] = nf
  m[11] = near * nf
  m[15] = 1
  return m
}

export function lookAt(eye: Vec3, center: Vec3, up: Vec3): Mat4 {
  let zx = eye[0] - center[0], zy = eye[1] - center[1], zz = eye[2] - center[2]
  let len = Math.hypot(zx, zy, zz)
  if (len < 1e-10) { zx = 0; zy = 0; zz = 1; len = 1 }
  const fz = [zx / len, zy / len, zz / len]

  let xx = up[1] * fz[2] - up[2] * fz[1]
  let xy = up[2] * fz[0] - up[0] * fz[2]
  let xz = up[0] * fz[1] - up[1] * fz[0]
  len = Math.hypot(xx, xy, xz)
  // Top and bottom views make a Z-up vector parallel to the view direction.
  // Pick a stable secondary up axis instead of producing NaNs.
  if (len < 1e-10) {
    const [ax, ay, az] = Math.abs(fz[2]) > 0.9 ? [0, 1, 0] : [0, 0, 1]
    xx = ay * fz[2] - az * fz[1]
    xy = az * fz[0] - ax * fz[2]
    xz = ax * fz[1] - ay * fz[0]
    len = Math.hypot(xx, xy, xz)
  }
  const fx = [xx / len, xy / len, xz / len]
  const fy = [
    fz[1] * fx[2] - fz[2] * fx[1],
    fz[2] * fx[0] - fz[0] * fx[2],
    fz[0] * fx[1] - fz[1] * fx[0],
  ]
  const m = new Float32Array(16)
  m[0] = fx[0]; m[1] = fx[1]; m[2] = fx[2]; m[3] = -(fx[0]*eye[0]+fx[1]*eye[1]+fx[2]*eye[2])
  m[4] = fy[0]; m[5] = fy[1]; m[6] = fy[2]; m[7] = -(fy[0]*eye[0]+fy[1]*eye[1]+fy[2]*eye[2])
  m[8] = fz[0]; m[9] = fz[1]; m[10] = fz[2]; m[11] = -(fz[0]*eye[0]+fz[1]*eye[1]+fz[2]*eye[2])
  m[12] = 0; m[13] = 0; m[14] = 0; m[15] = 1
  return m
}

export function transpose(m: Mat4): Mat4 {
  return new Float32Array(callGeometryRust<number[]>('viewport', { action: 'transpose', matrix: Array.from(m) }))
}

/** Native inverse. Singular matrices refuse; output remains untouched on failure. */
export function invert(a: Mat4, out?: Mat4): Mat4 {
  const values = callGeometryRust<number[]>('viewport', { action: 'inverse', matrix: Array.from(a) })
  if (out) { out.set(values); return out }
  return new Float32Array(values)
}

/** Transform a point by a row-major matrix, including homogeneous division. */
export function transformPoint(m: Mat4, point: Vec3): Vec3 {
  const [x, y, z] = point
  const w = m[12]*x + m[13]*y + m[14]*z + m[15]
  const iw = Number.isFinite(w) && Math.abs(w) > 1e-12 ? 1 / w : 1
  return [
    (m[0]*x + m[1]*y + m[2]*z + m[3]) * iw,
    (m[4]*x + m[5]*y + m[6]*z + m[7]) * iw,
    (m[8]*x + m[9]*y + m[10]*z + m[11]) * iw,
  ]
}

/** Transform a direction by the linear part of a row-major matrix. */
export function transformVector(m: Mat4, vector: Vec3): Vec3 {
  const [x, y, z] = vector
  return [
    m[0]*x + m[1]*y + m[2]*z,
    m[4]*x + m[5]*y + m[6]*z,
    m[8]*x + m[9]*y + m[10]*z,
  ]
}

/**
 * Build a world-space ray from WebGPU NDC coordinates. The supplied matrix is
 * the inverse of projection * view; WebGPU's near/far depth values are 0/1.
 */
export { unprojectRayInKernel as unprojectRay } from './geometry/viewport'

/** Return the first non-negative ray distance to an AABB, or null on a miss. */
export function rayAabbDistance(ray: Ray3, bounds: Aabb3, maxDistance = Infinity): number | null {
  let near = 0
  let far = maxDistance
  for (let axis = 0; axis < 3; axis++) {
    const origin = ray.origin[axis]
    const direction = ray.direction[axis]
    const min = bounds.min[axis]
    const max = bounds.max[axis]
    if (Math.abs(direction) < 1e-12) {
      if (origin < min || origin > max) return null
      continue
    }
    let a = (min - origin) / direction
    let b = (max - origin) / direction
    if (a > b) [a, b] = [b, a]
    near = Math.max(near, a)
    far = Math.min(far, b)
    if (far < near) return null
  }
  return near <= maxDistance ? near : null
}

/** Double-sided Moller-Trumbore ray/triangle intersection. */
export function rayTriangleDistance(ray: Ray3, a: Vec3, b: Vec3, c: Vec3): number | null {
  const e1x = b[0]-a[0], e1y = b[1]-a[1], e1z = b[2]-a[2]
  const e2x = c[0]-a[0], e2y = c[1]-a[1], e2z = c[2]-a[2]
  const px = ray.direction[1]*e2z - ray.direction[2]*e2y
  const py = ray.direction[2]*e2x - ray.direction[0]*e2z
  const pz = ray.direction[0]*e2y - ray.direction[1]*e2x
  const det = e1x*px + e1y*py + e1z*pz
  if (Math.abs(det) < 1e-10) return null
  const invDet = 1 / det
  const tx = ray.origin[0]-a[0], ty = ray.origin[1]-a[1], tz = ray.origin[2]-a[2]
  const u = (tx*px + ty*py + tz*pz) * invDet
  if (u < 0 || u > 1) return null
  const qx = ty*e1z - tz*e1y
  const qy = tz*e1x - tx*e1z
  const qz = tx*e1y - ty*e1x
  const v = (ray.direction[0]*qx + ray.direction[1]*qy + ray.direction[2]*qz) * invDet
  if (v < 0 || u + v > 1) return null
  const distance = (e2x*qx + e2y*qy + e2z*qz) * invDet
  return distance >= 0 && Number.isFinite(distance) ? distance : null
}

/**
 * Exact indexed-triangle hit test. The inverse model transform maps the world
 * ray into mesh-local space without normalizing its direction, preserving the
 * world-space distance parameter for affine transforms.
 */
export function rayIndexedMeshDistance(
  worldRay: Ray3,
  vertices: Float32Array,
  indices: Uint32Array,
  inverseModel: Mat4,
  localBounds: Aabb3,
  maxDistance = Infinity,
  vertexStride = 6,
): number | null {
  if (vertexStride < 3 || indices.length < 3) return null
  const ray: Ray3 = {
    origin: transformPoint(inverseModel, worldRay.origin),
    // Deliberately left unnormalized; see the function comment above.
    direction: transformVector(inverseModel, worldRay.direction),
  }
  if (rayAabbDistance(ray, localBounds, maxDistance) === null) return null

  let nearest = maxDistance
  let hit = false
  for (let i = 0; i + 2 < indices.length; i += 3) {
    const ia = indices[i] * vertexStride
    const ib = indices[i+1] * vertexStride
    const ic = indices[i+2] * vertexStride
    if (ic + 2 >= vertices.length || ib + 2 >= vertices.length || ia + 2 >= vertices.length) continue

    const ax = vertices[ia], ay = vertices[ia+1], az = vertices[ia+2]
    const e1x = vertices[ib]-ax, e1y = vertices[ib+1]-ay, e1z = vertices[ib+2]-az
    const e2x = vertices[ic]-ax, e2y = vertices[ic+1]-ay, e2z = vertices[ic+2]-az
    const px = ray.direction[1]*e2z - ray.direction[2]*e2y
    const py = ray.direction[2]*e2x - ray.direction[0]*e2z
    const pz = ray.direction[0]*e2y - ray.direction[1]*e2x
    const det = e1x*px + e1y*py + e1z*pz
    if (Math.abs(det) < 1e-10) continue
    const invDet = 1 / det
    const tx = ray.origin[0]-ax, ty = ray.origin[1]-ay, tz = ray.origin[2]-az
    const u = (tx*px + ty*py + tz*pz) * invDet
    if (u < 0 || u > 1) continue
    const qx = ty*e1z - tz*e1y
    const qy = tz*e1x - tx*e1z
    const qz = tx*e1y - ty*e1x
    const v = (ray.direction[0]*qx + ray.direction[1]*qy + ray.direction[2]*qz) * invDet
    if (v < 0 || u + v > 1) continue
    const distance = (e2x*qx + e2y*qy + e2z*qz) * invDet
    if (distance >= 0 && distance < nearest && Number.isFinite(distance)) {
      nearest = distance
      hit = true
    }
  }
  return hit ? nearest : null
}
