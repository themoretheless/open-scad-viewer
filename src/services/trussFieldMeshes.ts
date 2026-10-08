import {callGeometryRust} from './geometry/kernel'
import type {MeshData} from '../core/mesh'
import type {TrussInput, TrussResponse} from './trussAnalysis'
import {importedStlToMeshData} from './stlImport'

export const TRUSS_FORCE_COLORS = {
  compression: [0.15, 0.55, 0.95, 1],
  zero: [0.6, 0.6, 0.6, 1],
  tension: [0.95, 0.3, 0.2, 1],
} satisfies Record<string, [number, number, number, number]>

/** Fixed-size midpoint markers for signed axial force, not strength/utilization. */
export function trussFieldMeshes(model: TrussInput, result: TrussResponse, markerMm: number): MeshData[] {
  const positions=callGeometryRust<number[][]>('truss_force_markers',{nodes:model.nodesMm,members:model.members.map(m=>m.nodes),forces:result.axialForcesN,marker:markerMm})
  const groups={compression:positions[0],zero:positions[1],tension:positions[2]}
  return (Object.keys(groups) as (keyof typeof groups)[]).flatMap(kind => {
    const positions=new Float32Array(groups[kind])
    if (!positions.length) return []
    const mesh=importedStlToMeshData({positions, triangleCount:positions.length/9}, TRUSS_FORCE_COLORS[kind])
    if(mesh.indices.length!==positions.length/3)throw new Error('Marker geometry collapsed at display precision; increase the marker size.')
    return [mesh]
  })
}
