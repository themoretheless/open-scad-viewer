import type {MeshData} from '../core/mesh'
import type {LighteningOptions} from './solidLightening'
import type {TrussVector} from './trussAnalysis'
import {fitsClonedBufferBudget} from './clonedBufferBudget'

export type LatticeGraphMesh = Pick<MeshData, 'vertices' | 'indices' | 'transform'>
export interface NominalLatticeGraph {
  modelKind: 'nominal-bounding-box-axial'
  nodes: TrussVector[]
  edges: [number, number][]
}
export function checkLatticeGraphMeshInput(mesh: LatticeGraphMesh): void {
  const meshError='Nominal lattice analysis requires a triangle mesh within 100000 triangles and 16 MiB of owned buffers.'
  if (!mesh || !(mesh.vertices instanceof Float32Array) || !(mesh.indices instanceof Uint32Array)
    || !(mesh.transform instanceof Float32Array) || !mesh.vertices.length || mesh.vertices.length % 6
    || !mesh.indices.length || mesh.indices.length % 3 || mesh.transform.length !== 16
    || mesh.indices.length > 300_000) {
    throw new Error(meshError)
  }
  if (!fitsClonedBufferBudget([mesh.vertices, mesh.indices, mesh.transform], 16 * 1024 * 1024)) throw new Error(meshError)
  if (!mesh.transform.every(Number.isFinite) || mesh.transform[12] !== 0 || mesh.transform[13] !== 0 || mesh.transform[14] !== 0 || mesh.transform[15] !== 1) {
    throw new Error('Nominal graph placement must be finite and affine.')
  }
}
export function checkLatticeGraphInput(mesh: LatticeGraphMesh, options: LighteningOptions): void {
  checkLatticeGraphMeshInput(mesh)
  if (!options || !['spatial','bone','bcc','octet'].includes(options.pattern)) throw new Error('Select a spatial lattice pattern.')
  if (!Number.isFinite(options.cell) || options.cell <= 0 || !Number.isFinite(options.jitter) || options.jitter < 0 || options.jitter > 1
    || !Number.isFinite(options.seed) || !Number.isInteger(options.seed)
    || (options.diagonals !== undefined && typeof options.diagonals !== 'boolean')) {
    throw new Error('Spatial graph requires positive cell size, jitter between 0 and 1, and an integer seed.')
  }
}
export function isNominalLatticeGraph(value: unknown): value is NominalLatticeGraph {
  if (!value || typeof value !== 'object') return false
  const graph = value as NominalLatticeGraph
  if (graph.modelKind !== 'nominal-bounding-box-axial' || !Array.isArray(graph.nodes)
    || !graph.nodes.length || graph.nodes.length > 125 || !Array.isArray(graph.edges)
    || !graph.edges.length || graph.edges.length > 400) return false
  const coordinates = new Set<string>(), members = new Set<string>()
  for (const node of graph.nodes) {
    if (!Array.isArray(node) || node.length !== 3 || ![...node].every(Number.isFinite)) return false
    const key = node.join(',')
    if (coordinates.has(key)) return false
    coordinates.add(key)
  }
  for (let k = 0; k < 3; k++) {
    const values = graph.nodes.map(node => node[k])
    if (!(Math.max(...values) > Math.min(...values))) return false
  }
  for (const edge of graph.edges) {
    if (!Array.isArray(edge) || edge.length !== 2 || ![...edge].every(index => Number.isInteger(index) && index >= 0 && index < graph.nodes.length)
      || edge[0] === edge[1]) return false
    const key = `${Math.min(...edge)},${Math.max(...edge)}`
    if (members.has(key)) return false
    members.add(key)
  }
  return true
}
