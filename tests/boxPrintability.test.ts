import {beforeAll,it,expect} from 'vitest'
import {GeometryBuildEngine} from '../src/kernel/geometryBuildEngine'
import {BOX_DEFAULTS,generateBox} from '../src/services/boxGenerator'
const engine=new GeometryBuildEngine(), request={quality:'full',purpose:'analysis'} as const
beforeAll(()=>engine.initializeSource('cube(1);',request),120000)
// Independent mesh-face audit in the declared print orientation (+Z).
// The build-plate contact faces are supported; every other downward face counts.
async function audit(p:typeof BOX_DEFAULTS) {
 const {result}=await engine.buildSource(generateBox(p),request)
 let worst=0, excessArea=0, downArea=0, microWidth=0, criticalWorst=0
 const bad:Array<{area:number;angle:number;points:number[][]}>=[]
 for(const mesh of result.meshes) for(let i=0;i<mesh.indices.length;i+=3) {
  const a=mesh.indices[i]*6,b=mesh.indices[i+1]*6,c=mesh.indices[i+2]*6,v=mesh.vertices
  if(Math.max(v[a+2],v[b+2],v[c+2])<0.001)continue
  const ux=v[b]-v[a],uy=v[b+1]-v[a+1],uz=v[b+2]-v[a+2]
  const vx=v[c]-v[a],vy=v[c+1]-v[a+1],vz=v[c+2]-v[a+2]
  const nx=uy*vz-uz*vy,ny=uz*vx-ux*vz,nz=ux*vy-uy*vx
  const size=Math.hypot(nx,ny,nz)
  if(nz>=-1e-8||size<1e-6)continue
  const angle=Math.asin(Math.min(1,-nz/size))*180/Math.PI
  worst=Math.max(worst,angle);downArea+=size/2
  if(angle>p.overhang_angle+0.2){
   const maxEdge=Math.max(Math.hypot(ux,uy,uz),Math.hypot(vx,vy,vz),Math.hypot(v[c]-v[b],v[c+1]-v[b+1],v[c+2]-v[b+2]))
   const minWidth=size/maxEdge
   // One 0.4 mm extrusion is the screening bridge allowance, not a printer guarantee.
   if(minWidth<=0.4){microWidth=Math.max(microWidth,minWidth);continue}
   criticalWorst=Math.max(criticalWorst,angle);excessArea+=size/2
   bad.push({area:size/2,angle,points:[a,b,c].map(at=>Array.from(v.slice(at,at+3)))})
  }
 }
 return {worst,criticalWorst,microWidth,excessArea,downArea,volume:result.volume,bad:bad.sort((a,b)=>b.area-a.area).slice(0,4)}
}
it.each([
 {},{pattern_seed:7},{overhang_angle:35},{corner_radius:12,front_angle:15},
 {bend_radius:140,corner_radius:12,front_angle:15},{bend_radius:100,corner_radius:12,front_angle:15},
 {bend_radius:200,bend_reverse:true,front_angle:-15},
 {organic:false,hole_diameter:8,hole_pitch:18,overhang_angle:45},
])('avoids over-limit wall undersides wider than one extrusion: %j',async patch=>{
 const p={...BOX_DEFAULTS,hooks:false,...patch}
 const report=await audit(p)
 expect(report.excessArea,JSON.stringify(report)).toBeLessThan(0.02)
 expect(report.criticalWorst,JSON.stringify(report)).toBeLessThanOrEqual(p.overhang_angle+0.2)
 if(!p.bend_radius)expect(report.worst).toBeLessThanOrEqual(p.overhang_angle+0.2)
},120000)
it('keeps default organic wall overhang well below the 45-degree limit',async()=>{
 const report=await audit({...BOX_DEFAULTS,hooks:false})
 expect(report.worst).toBeLessThan(43)
 expect(report.excessArea).toBe(0)
},120000)
it('detects the old rounded ceilings as a failing control',async()=>{
 const report=await audit({...BOX_DEFAULTS,hooks:false,print_optimized:false})
 expect(report.excessArea).toBeGreaterThan(10)
 expect(report.worst).toBeGreaterThan(70)
},120000)

it('keeps the downturned-hook support risk explicit instead of claiming the whole box is support-free',async()=>{
 const report=await audit({...BOX_DEFAULTS})
 expect(report.excessArea).toBeGreaterThan(10)
 expect(report.worst).toBeGreaterThan(80)
 // Enabling the 45-degree root gusset (hook_chamfer) eliminates the horizontal bridge ceiling and root step,
 // reducing the unsupported >45° area to less than a third of the plain rectangular hook.
 const chamfered=await audit({...BOX_DEFAULTS,hook_chamfer:true})
 expect(chamfered.excessArea).toBeLessThan(report.excessArea*0.3)
},120000)
it('rejects a wall tilt beyond the selected print angle',()=>{
 expect(()=>generateBox({...BOX_DEFAULTS,front_angle:40,overhang_angle:35})).toThrow(/свеса/)
 expect(()=>generateBox({...BOX_DEFAULTS,front_angle:40,overhang_angle:35,print_optimized:false})).not.toThrow()
})
