import { identity, type Mat4 } from './math3d'

type Parts = { position: number[]; scale: number[]; rotation: number[] }
function decompose(m: Mat4): Parts | null {
  const scale = [Math.hypot(m[0], m[4], m[8]), Math.hypot(m[1], m[5], m[9]), Math.hypot(m[2], m[6], m[10])]
  if (scale.some(s => s < 1e-8) || Math.abs(m[12]) + Math.abs(m[13]) + Math.abs(m[14]) + Math.abs(m[15] - 1) > 1e-6) return null
  const r = Array.from({ length: 9 }, (_, i) => m[Math.floor(i / 3) * 4 + i % 3] / scale[i % 3])
  const det = r[0] * (r[4] * r[8] - r[5] * r[7]) - r[1] * (r[3] * r[8] - r[5] * r[6]) + r[2] * (r[3] * r[7] - r[4] * r[6])
  if (det < 0) { scale[0] *= -1; r[0] *= -1; r[3] *= -1; r[6] *= -1 }
  for (let a = 0; a < 3; a++) for (let b = a + 1; b < 3; b++) if (Math.abs(r[a] * r[b] + r[a + 3] * r[b + 3] + r[a + 6] * r[b + 6]) > 1e-5) return null
  let q: number[]
  const trace = r[0] + r[4] + r[8]
  if (trace > 0) {
    const s = Math.sqrt(trace + 1) * 2
    q = [(r[7] - r[5]) / s, (r[2] - r[6]) / s, (r[3] - r[1]) / s, s / 4]
  } else {
    const i = r[0] > r[4] && r[0] > r[8] ? 0 : r[4] > r[8] ? 1 : 2
    const j = (i + 1) % 3, k = (i + 2) % 3
    const s = Math.sqrt(1 + r[i * 3 + i] - r[j * 3 + j] - r[k * 3 + k]) * 2
    q = [0, 0, 0, 0]; q[i] = s / 4; q[j] = (r[j * 3 + i] + r[i * 3 + j]) / s; q[k] = (r[k * 3 + i] + r[i * 3 + k]) / s; q[3] = (r[k * 3 + j] - r[j * 3 + k]) / s
  }
  return { position: [m[3], m[7], m[11]], scale, rotation: q }
}

/** TRS interpolation keeps rotations rigid; shear/reflection changes use a visual dissolve. */
export function geometryTransformTransition(from: Mat4, to: Mat4): ((t: number) => Mat4) | null {
  const a = decompose(from), b = decompose(to)
  if (!a || !b || a.scale.some((s, i) => s * b.scale[i] <= 0)) return null
  let dot = a.rotation[0] * b.rotation[0] + a.rotation[1] * b.rotation[1] + a.rotation[2] * b.rotation[2] + a.rotation[3] * b.rotation[3]
  const bx = dot < 0 ? -b.rotation[0] : b.rotation[0]
  const by = dot < 0 ? -b.rotation[1] : b.rotation[1]
  const bz = dot < 0 ? -b.rotation[2] : b.rotation[2]
  const bw = dot < 0 ? -b.rotation[3] : b.rotation[3]
  if (dot < 0) dot = -dot
  const ax = a.rotation[0], ay = a.rotation[1], az = a.rotation[2], aw = a.rotation[3]
  const angle = Math.acos(Math.min(1, dot))
  const sin = Math.sin(angle)
  const as0 = a.scale[0], as1 = a.scale[1], as2 = a.scale[2]
  const ds0 = b.scale[0] - as0, ds1 = b.scale[1] - as1, ds2 = b.scale[2] - as2
  const ap0 = a.position[0], ap1 = a.position[1], ap2 = a.position[2]
  const dp0 = b.position[0] - ap0, dp1 = b.position[1] - ap1, dp2 = b.position[2] - ap2
  return t => {
    if (t <= 0) return new Float32Array(from)
    if (t >= 1) return new Float32Array(to)
    const wa = sin < 1e-5 ? 1 - t : Math.sin((1 - t) * angle) / sin
    const wb = sin < 1e-5 ? t : Math.sin(t * angle) / sin
    const qx = wa * ax + wb * bx
    const qy = wa * ay + wb * by
    const qz = wa * az + wb * bz
    const qw = wa * aw + wb * bw
    const invLen = 1 / Math.hypot(qx, qy, qz, qw)
    const x = qx * invLen, y = qy * invLen, z = qz * invLen, w = qw * invLen
    const s0 = as0 + ds0 * t, s1 = as1 + ds1 * t, s2 = as2 + ds2 * t
    const result = new Float32Array(16)
    result[0] = (1 - 2 * (y * y + z * z)) * s0
    result[1] = 2 * (x * y - z * w) * s1
    result[2] = 2 * (x * z + y * w) * s2
    result[3] = ap0 + dp0 * t
    result[4] = 2 * (x * y + z * w) * s0
    result[5] = (1 - 2 * (x * x + z * z)) * s1
    result[6] = 2 * (y * z - x * w) * s2
    result[7] = ap1 + dp1 * t
    result[8] = 2 * (x * z - y * w) * s0
    result[9] = 2 * (y * z + x * w) * s1
    result[10] = (1 - 2 * (x * x + y * y)) * s2
    result[11] = ap2 + dp2 * t
    result[15] = 1
    return result
  }
}
