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
  if (!Number.isFinite(markerMm) || markerMm <= 0 || !model.nodesMm.length || model.nodesMm.length > 125
    || !model.members.length || model.members.length > 400 || result.axialForcesN.length !== model.members.length
    || !result.axialForcesN.every(Number.isFinite)
    || !model.nodesMm.every(point => point.length === 3 && point.every(Number.isFinite))) throw new Error('Invalid axial force field.')
  const groups = {compression: [] as number[], zero: [] as number[], tension: [] as number[]}
  const faces = [[0,2,4],[0,4,3],[0,3,5],[0,5,2],[1,4,2],[1,3,4],[1,5,3],[1,2,5]]
  for (let i=0;i<model.members.length;i++) {
    const pair=model.members[i].nodes
    if (pair.length !== 2 || pair[0] === pair[1] || !pair.every(index => Number.isInteger(index) && index >= 0 && index < model.nodesMm.length)) throw new Error('Invalid axial force member.')
    const a=model.nodesMm[pair[0]], b=model.nodesMm[pair[1]]
    const mid=a.map((value,k)=>value/2+b[k]/2), r=markerMm/2
    const points=[[mid[0]+r,mid[1],mid[2]],[mid[0]-r,mid[1],mid[2]],
      [mid[0],mid[1]+r,mid[2]],[mid[0],mid[1]-r,mid[2]],
      [mid[0],mid[1],mid[2]+r],[mid[0],mid[1],mid[2]-r]]
    const force=result.axialForcesN[i], group=groups[force<0?'compression':force>0?'tension':'zero']
    for (const face of faces) for (const vertex of face) group.push(...points[vertex])
  }
  return (Object.keys(groups) as (keyof typeof groups)[]).flatMap(kind => {
    const positions=new Float32Array(groups[kind])
    if (!positions.length) return []
    return [importedStlToMeshData({positions, triangleCount:positions.length/9}, TRUSS_FORCE_COLORS[kind])]
  })
}
