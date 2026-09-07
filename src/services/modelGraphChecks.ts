import type { MeshData } from '../core/mesh'
import { flattenExportMeshes } from './meshExportAdapter'

export interface GeometryCheck {
  id: string
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
export function checkModelGraphGeometry(checks: readonly GeometryCheck[], meshes: readonly MeshData[]): GeometryCheckResult[] {
  if (!checks.length) return []
  try {
    const mesh = flattenExportMeshes(meshes)
    let bodies = 0
    for (const part of mesh.parts) {
      const parent = Array.from({ length: part.positions.length / 3 }, (_, i) => i)
      const find = (v: number): number => { while (parent[v] !== v) { parent[v] = parent[parent[v]!]!; v = parent[v]! } return v }
      const used = new Set<number>()
      for (let i = 0; i < part.indices.length; i += 3) {
        const a = part.indices[i]!, b = part.indices[i + 1]!, c = part.indices[i + 2]!
        parent[find(b)] = find(a); parent[find(c)] = find(a)
        used.add(a); used.add(b); used.add(c)
      }
      bodies += new Set([...used].map(find)).size
    }
    const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity]
    for (const index of mesh.indices) for (let axis = 0; axis < 3; axis++) {
      const v = mesh.positions[index * 3 + axis]!
      min[axis] = Math.min(min[axis]!, v); max[axis] = Math.max(max[axis]!, v)
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
export class ModelGraphCheckError extends Error {
  constructor(readonly checks: readonly GeometryCheckResult[]) {
    super('Geometry assertions: ' + JSON.stringify(checks))
  }
}
export function requireModelGraphChecks(report: readonly GeometryCheckResult[]) {
  if (report.some(check => check.status !== 'passed')) throw new ModelGraphCheckError(report)
}
