// Isolated browser probe for the WebGPU sweep runner: a synthetic shifted-plane
// scene with known answer (best hypothesis bin 10 of 16, z=4), mirroring the
// kernel's gpu_sweep unit test.
import { runGpuSweep } from '../../src/services/photoGpuSweep'

function buildBlob(): Uint8Array {
  // Two 80x80 grays; source is the reference shifted left by 10 px.
  let rng = 0xABCDEFn
  const next = () => {
    rng = BigInt.asUintN(64, rng * 6364136223846793005n + 1442695040888963407n)
    return Number(rng >> 33n) / 2147483648
  }
  const tex = new Float32Array(80 * 80).map(() => next())
  const src = new Float32Array(80 * 80)
  for (let y = 0; y < 80; y++) for (let x = 0; x < 70; x++) src[y * 80 + x] = tex[y * 80 + x + 10]

  const floats: number[] = []
  const out = new Uint8Array(1024 * 1024)
  const view = new DataView(out.buffer)
  let at = 0
  const u32 = (v: number) => { view.setUint32(at, v >>> 0, true); at += 4 }
  const f32 = (v: number) => { view.setFloat32(at, v, true); at += 4 }
  u32(0x31505753); u32(2); u32(16); u32(1) // magic, images, hypotheses, radius
  u32(80); u32(80); u32(80); u32(80) // image headers
  for (const v of tex) f32(v)
  for (const v of src) f32(v)
  u32(2) // view count
  // View 0 (reference image 0) present, one source (image 1).
  u32(1); u32(40); u32(40); u32(1); u32(1); u32(0)
  f32(2); f32(80); f32(40); f32(40) // step, ref_f, ref_cx, ref_cy
  const near = 2, far = 8, last = 15
  for (let d = 0; d < 16; d++) {
    const f = d / last
    f32(1 / ((1 - f) / near + f / far))
  }
  // Source: identity rotation, translation [-0.5, 0, 0], f=80, cx=cy=40.
  const srcFloats = [1, 0, 0, 0, 1, 0, 0, 0, 1, -0.5, 0, 0, 80, 40, 40, 0]
  for (const v of srcFloats) f32(v)
  u32(1); u32(0); u32(0); u32(0)
  // View 1 absent.
  u32(0)
  void floats
  return out.slice(0, at)
}

export async function probe(): Promise<string> {
  const blob = buildBlob()
  const wgsl = await (await fetch('/sweep.wgsl')).text()
  const scores = await runGpuSweep(blob, wgsl)
  const map = 40 * 40, n = 16
  const pixel = 20 * 40 + 20
  const row = Array.from(scores.slice(pixel * n, pixel * n + n))
  const best = row.indexOf(Math.max(...row))
  return `pixel(20,20) best bin: ${best} (expected 10), score: ${row[best]?.toFixed(4)}, ` +
    `row: ${row.map(v => v.toFixed(2)).join(' ')}`
}
