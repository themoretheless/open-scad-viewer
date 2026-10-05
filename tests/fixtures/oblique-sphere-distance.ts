import {readFileSync} from 'node:fs'
const distanceCases=JSON.parse(readFileSync(new URL('../../docs/qualification/cad-roadmap-2026-09-28/solid-distance-2026-09-30/contract-fixtures.json',import.meta.url),'utf8')).cases
export async function obliqueSphereRequests(){
 const {callGeometryRust,warmGeometryKernel}=await import('../../src/services/geometry/kernel')
 await warmGeometryKernel()
 const a=callGeometryRust<any>('brep_nurbs_sphere',{radius:3})
 return [[4,4,4],[8,1,2],[6+2**-20,0,0],[0,0,6+2**-20]].map(offset=>{
  const b=structuredClone(a)
  const shift=(p:number[])=>{for(let i=0;i<3;i++)p[i]+=offset[i]}
  b.vertices.forEach((v:any)=>shift(v.point))
  b.edges.forEach((e:any)=>e.curve.controlPoints.forEach(shift))
  b.faces.forEach((f:any)=>f.surface.controlPoints.forEach((row:number[][])=>row.forEach(shift)))
  return {options:{...structuredClone(distanceCases[6].request),a,b,toleranceMm:1e-5},expected:Math.hypot(...offset)-6}
 })
}
