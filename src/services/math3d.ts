/** Minimal 3D math: mat4 (row-major) + vec3 */

export type Mat4 = Float32Array
export type Vec3 = [number, number, number]

export function identity(): Mat4 {
  const m = new Float32Array(16)
  m[0] = m[5] = m[10] = m[15] = 1
  return m
}

export function multiply(a: Mat4, b: Mat4): Mat4 {
  const r = new Float32Array(16)
  for (let i = 0; i < 4; i++)
    for (let j = 0; j < 4; j++) {
      let s = 0
      for (let k = 0; k < 4; k++) s += a[i * 4 + k] * b[k * 4 + j]
      r[i * 4 + j] = s
    }
  return r
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
  m[10] = (far + near) * nf; m[11] = 2 * far * near * nf
  m[14] = -1
  return m
}

export function lookAt(eye: Vec3, center: Vec3, up: Vec3): Mat4 {
  const zx = eye[0] - center[0], zy = eye[1] - center[1], zz = eye[2] - center[2]
  let len = 1 / Math.sqrt(zx * zx + zy * zy + zz * zz)
  const fz = [zx * len, zy * len, zz * len]
  const xx = up[1] * fz[2] - up[2] * fz[1]
  const xy = up[2] * fz[0] - up[0] * fz[2]
  const xz = up[0] * fz[1] - up[1] * fz[0]
  len = 1 / Math.sqrt(xx * xx + xy * xy + xz * xz)
  const fx = [xx * len, xy * len, xz * len]
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
  const r = new Float32Array(16)
  for (let i = 0; i < 4; i++)
    for (let j = 0; j < 4; j++) r[j * 4 + i] = m[i * 4 + j]
  return r
}

export function invert(a: Mat4): Mat4 {
  const inv = new Float32Array(16)
  inv[0]  =  a[5]*a[10]*a[15] - a[5]*a[11]*a[14] - a[9]*a[6]*a[15] + a[9]*a[7]*a[14] + a[13]*a[6]*a[11] - a[13]*a[7]*a[10]
  inv[4]  = -a[4]*a[10]*a[15] + a[4]*a[11]*a[14] + a[8]*a[6]*a[15] - a[8]*a[7]*a[14] - a[12]*a[6]*a[11] + a[12]*a[7]*a[10]
  inv[8]  =  a[4]*a[9]*a[15]  - a[4]*a[11]*a[13] - a[8]*a[5]*a[15] + a[8]*a[7]*a[13] + a[12]*a[5]*a[11] - a[12]*a[7]*a[9]
  inv[12] = -a[4]*a[9]*a[14]  + a[4]*a[10]*a[13] + a[8]*a[5]*a[14] - a[8]*a[6]*a[13] - a[12]*a[5]*a[10] + a[12]*a[6]*a[9]
  inv[1]  = -a[1]*a[10]*a[15] + a[1]*a[11]*a[14] + a[9]*a[2]*a[15] - a[9]*a[3]*a[14] - a[13]*a[2]*a[11] + a[13]*a[3]*a[10]
  inv[5]  =  a[0]*a[10]*a[15] - a[0]*a[11]*a[14] - a[8]*a[2]*a[15] + a[8]*a[3]*a[14] + a[12]*a[2]*a[11] - a[12]*a[3]*a[10]
  inv[9]  = -a[0]*a[9]*a[15]  + a[0]*a[11]*a[13] + a[8]*a[1]*a[15] - a[8]*a[3]*a[13] - a[12]*a[1]*a[11] + a[12]*a[3]*a[9]
  inv[13] =  a[0]*a[9]*a[14]  - a[0]*a[10]*a[13] - a[8]*a[1]*a[14] + a[8]*a[2]*a[13] + a[12]*a[1]*a[10] - a[12]*a[2]*a[9]
  inv[2]  =  a[1]*a[6]*a[15]  - a[1]*a[7]*a[14]  - a[5]*a[2]*a[15] + a[5]*a[3]*a[14] + a[13]*a[2]*a[7]  - a[13]*a[3]*a[6]
  inv[6]  = -a[0]*a[6]*a[15]  + a[0]*a[7]*a[14]  + a[4]*a[2]*a[15] - a[4]*a[3]*a[14] - a[12]*a[2]*a[7]  + a[12]*a[3]*a[6]
  inv[10] =  a[0]*a[5]*a[15]  - a[0]*a[7]*a[13]  - a[4]*a[1]*a[15] + a[4]*a[3]*a[13] + a[12]*a[1]*a[7]  - a[12]*a[3]*a[5]
  inv[14] = -a[0]*a[5]*a[14]  + a[0]*a[6]*a[13]  + a[4]*a[1]*a[14] - a[4]*a[2]*a[13] - a[12]*a[1]*a[6]  + a[12]*a[2]*a[5]
  inv[3]  = -a[1]*a[6]*a[11]  + a[1]*a[7]*a[10]  + a[5]*a[2]*a[11] - a[5]*a[3]*a[10] - a[9]*a[2]*a[7]   + a[9]*a[3]*a[6]
  inv[7]  =  a[0]*a[6]*a[11]  - a[0]*a[7]*a[10]  - a[4]*a[2]*a[11] + a[4]*a[3]*a[10] + a[8]*a[2]*a[7]   - a[8]*a[3]*a[6]
  inv[11] = -a[0]*a[5]*a[11]  + a[0]*a[7]*a[9]   + a[4]*a[1]*a[11] - a[4]*a[3]*a[9]  - a[8]*a[1]*a[7]   + a[8]*a[3]*a[5]
  inv[15] =  a[0]*a[5]*a[10]  - a[0]*a[6]*a[9]   - a[4]*a[1]*a[10] + a[4]*a[2]*a[9]  + a[8]*a[1]*a[6]   - a[8]*a[2]*a[5]
  let det = a[0]*inv[0] + a[1]*inv[4] + a[2]*inv[8] + a[3]*inv[12]
  if (Math.abs(det) < 1e-10) return identity()
  det = 1 / det
  for (let i = 0; i < 16; i++) inv[i] *= det
  return inv
}
