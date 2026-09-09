// Isolated browser probe for runSdfSweep: a unit-sphere field sampled on a
// coarse grid; center must be deeply negative, far corner positive.
import { runSdfSweep, type SdfGpuPayload } from '../../src/services/sdfGpu'

export async function probe(): Promise<string> {
  const wgsl = await (await fetch('/sdf.wgsl')).text()
  const payload: SdfGpuPayload = {
    id: 1,
    kinds: [0], // sphere
    params: [0, 0, 0, 10, 0, 0, 0, 0],
    aux: [0, 0],
    triangles: [],
    min: [-12, -12, -12],
    max: [12, 12, 12],
    cells: [8, 8, 8],
    wgsl,
  }
  const values = await runSdfSweep(payload)
  const row = 9
  const at = (x: number, y: number, z: number) => values[(z * row + y) * row + x]!
  const center = at(4, 4, 4)
  const corner = at(0, 0, 0)
  return `center=${center.toFixed(3)} (expect ≈ -10), corner=${corner.toFixed(3)} (expect > 0), total=${values.length}`
}
