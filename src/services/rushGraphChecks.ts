import type { MeshData } from '../core/mesh'
import { flattenExportMeshes } from './meshExportAdapter'

export interface GeometryCheck {
  id: string
  target?: string
  instance_path?: string
  source?: string
  check: string
  expected: number
  tolerance: number
  message: string
}
export interface GeometryCheckResult extends GeometryCheck {
  actual: number | null
  status: 'passed' | 'failed' | 'unknown'
  reason?: string
}
/** Body count is connected triangle components within each scene mesh, with exact position welding. */
export function checkRushGraphGeometry(checks: readonly GeometryCheck[], meshes: readonly MeshData[]): GeometryCheckResult[] {
  if (!checks.length) return []
  try {
    const mesh = flattenExportMeshes(meshes)
    let bodies = 0
    for (const part of mesh.parts) {
      const vCount = part.positions.length / 3
      const parent = new Int32Array(vCount)
      for (let i = 0; i < vCount; i++) parent[i] = i
      const used = new Uint8Array(vCount)
      const find = (v: number): number => { while (parent[v] !== v) { parent[v] = parent[parent[v]]; v = parent[v] } return v }
      for (let i = 0; i < part.indices.length; i += 3) {
        const a = part.indices[i]!, b = part.indices[i + 1]!, c = part.indices[i + 2]!
        const ra = find(a)
        parent[find(b)] = ra; parent[find(c)] = ra
        used[a] = 1; used[b] = 1; used[c] = 1
      }
      for (let i = 0; i < vCount; i++) {
        if (used[i] && parent[i] === i) bodies++
      }
    }
    const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity]
    const pos = mesh.positions, idx = mesh.indices
    for (let i = 0; i < idx.length; i++) {
      const base = idx[i]! * 3
      const vx = pos[base]!, vy = pos[base + 1]!, vz = pos[base + 2]!
      if (vx < min[0]) min[0] = vx; if (vx > max[0]) max[0] = vx
      if (vy < min[1]) min[1] = vy; if (vy > max[1]) max[1] = vy
      if (vz < min[2]) min[2] = vz; if (vz > max[2]) max[2] = vz
    }
    return checks.map(check => {
      let actual: number | null = null
      if (check.check === 'hasBodies') actual = bodies
      if (check.check === 'isWatertight') actual = mesh.parts.length && mesh.parts.every(p => p.report.closed) ? 0 : 1
      if (check.check === 'hasNoDegenerateTriangles') actual = mesh.parts.reduce((n, p) => n + p.report.degenerateTriangles, 0)
      const axis = ['width', 'depth', 'height'].indexOf(check.check)
      if (axis >= 0 && mesh.indices.length) actual = max[axis]! - min[axis]!
      return { ...check, actual, status: actual === null ? 'unknown' : Math.abs(actual - check.expected) <= check.tolerance ? 'passed' : 'failed' }
    })
  } catch (error) {
    return checks.map(check => ({ ...check, actual: null, status: 'unknown', reason: error instanceof Error ? error.message : 'Measurement unavailable' }))
  }
}
/** Resolve each assertion against the exact target snapshot emitted by the runtime. */
export async function evaluateRushGraphGeometry(
  checks: readonly GeometryCheck[],
  meshes: readonly MeshData[],
  buildTarget: (source: string) => Promise<readonly MeshData[]>,
): Promise<GeometryCheckResult[]> {
  const groups = new Map<string | undefined, {check:GeometryCheck; index:number}[]>()
  checks.forEach(({source, ...check}, index) => {
    const group = groups.get(source) ?? []
    group.push({check, index})
    groups.set(source, group)
  })
  const report: GeometryCheckResult[] = new Array(checks.length)
  await Promise.all(Array.from(groups, async ([source, group]) => {
    let results: GeometryCheckResult[]
    try {
      const target = source === undefined ? meshes : await buildTarget(source)
      results = checkRushGraphGeometry(group.map(item => item.check), target)
    } catch (error) {
      results = group.map(({check}) => ({...check, actual:null, status:'unknown', reason:error instanceof Error ? error.message : 'Assertion target could not be built'}))
    }
    group.forEach(({index}, offset) => { report[index] = results[offset]! })
  }))
  return report
}

export class RushGraphCheckError extends Error {
  constructor(readonly checks: readonly GeometryCheckResult[]) {
    super('Geometry assertions: ' + JSON.stringify(checks))
  }
}
export function requireRushGraphChecks(report: readonly GeometryCheckResult[]) {
  if (report.some(check => check.status !== 'passed')) throw new RushGraphCheckError(report)
}
