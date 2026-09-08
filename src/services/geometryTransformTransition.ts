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
  let dot = a.rotation.reduce((sum, v, i) => sum + v * b.rotation[i], 0)
  if (dot < 0) { b.rotation = b.rotation.map(v => -v); dot = -dot }
  return t => {
    if (t <= 0) return new Float32Array(from)
    if (t >= 1) return new Float32Array(to)
    const angle = Math.acos(Math.min(1, dot)), sin = Math.sin(angle)
    const wa = sin < 1e-5 ? 1 - t : Math.sin((1 - t) * angle) / sin
    const wb = sin < 1e-5 ? t : Math.sin(t * angle) / sin
    const q = a.rotation.map((v, i) => wa * v + wb * b.rotation[i]); const length = Math.hypot(...q)
    const [x, y, z, w] = q.map(v => v / length)
    const r = [1-2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w), 2*(x*y+z*w), 1-2*(x*x+z*z), 2*(y*z-x*w), 2*(x*z-y*w), 2*(y*z+x*w), 1-2*(x*x+y*y)]
    const result = identity()
    for (let row = 0; row < 3; row++) {
      for (let col = 0; col < 3; col++) result[row*4+col] = r[row*3+col] * (a.scale[col] + (b.scale[col]-a.scale[col])*t)
      result[row*4+3] = a.position[row] + (b.position[row]-a.position[row])*t
    }
    return result
  }
}
