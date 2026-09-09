import {callGeometryRust} from '../src/services/geometryRustKernel'
// SDF-heavy probe: smooth-union of sphere+box tessellated at increasing grids.
const field={kind:'smooth_union',a:{kind:'sphere',center:[0,0,0],radius:10},b:{kind:'box',center:[8,0,0],half_size:[6,6,6]},radius:3}
const samples=[]
for(let i=0;i<5;i++){
  const start=performance.now()
  const r=callGeometryRust('sdf_tessellate',{field,grid:{min:[-12,-12,-12],max:[16,12,12],cells:[64,64,64]}})
  samples.push(performance.now()-start)
  if(i===0)console.log('triangles:',(r as any).mesh?.indices?.length/3 ?? (r as any).indices?.length/3)
}
console.log('sdf_tessellate 64^3 median ms:',samples.slice(1).sort((a,b)=>a-b)[1])
