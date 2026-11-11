import {beforeAll,it,expect} from 'vitest'
import {GeometryBuildEngine} from '../src/kernel/geometryBuildEngine'
import {extractSemanticEdges} from '../src/kernel/meshTopology'
import type {GeometryEvaluationResult} from '../src/core/build'
import {BOX_DEFAULTS,generateBox} from '../src/services/boxGenerator'
const engine=new GeometryBuildEngine(),request={quality:'full',purpose:'analysis'} as const
beforeAll(()=>engine.initializeSource('cube(1);',request),120000)
const build=async(p:typeof BOX_DEFAULTS)=>(await engine.buildSource(generateBox(p),request)).result
function closed(result:GeometryEvaluationResult){
 expect(result.meshes.length).toBe(1)
 for(const mesh of result.meshes){
  const d=extractSemanticEdges(mesh.vertices,mesh.indices).diagnostics
  expect(d.boundary).toBe(0);expect(d.nonManifold).toBe(0)
 }
}
function bedArea(result:GeometryEvaluationResult){
 let area=0
 for(const m of result.meshes)for(let i=0;i<m.indices.length;i+=3){
  const a=m.indices[i]*6,b=m.indices[i+1]*6,c=m.indices[i+2]*6,v=m.vertices
  if(Math.max(Math.abs(v[a+2]),Math.abs(v[b+2]),Math.abs(v[c+2]))>0.001)continue
  area+=Math.abs((v[b]-v[a])*(v[c+1]-v[a+1])-(v[b+1]-v[a+1])*(v[c]-v[a]))/2
 }
 return area
}
it('makes actual through-cells in the floor with closed ribs',async()=>{
 const p={...BOX_DEFAULTS,hooks:false}
 const solid=await build({...p,floor_mesh:false}),grid=await build(p)
 closed(grid)
 const openingArea=bedArea(solid)-bedArea(grid)
 expect(openingArea).toBeGreaterThan(1000)
 expect(solid.volume-grid.volume).toBeCloseTo(openingArea*p.floor,1)
},120000)
it('changes floor cell size and rib thickness independently from the wall grid',async()=>{
 const p={...BOX_DEFAULTS,width:60,depth:40,height:40,hooks:false,floor_cell_size:16}
 const base=await build(p),large=await build({...p,floor_cell_size:24}),thick=await build({...p,floor_rib:3.2})
 for(const result of [base,large,thick])closed(result)
 expect(Math.abs(base.volume-large.volume)).toBeGreaterThan(10)
 expect(thick.volume).toBeGreaterThan(base.volume)
},120000)
it.each([{wall:2,hook_thickness:4},{wall:5,hook_thickness:2}])('keeps hook dimensions independent of wall thickness: %j',async patch=>{
 const p={...BOX_DEFAULTS,...patch,porous:false,floor_mesh:false,hook_width:12,hook_gap:8,hook_drop:20,hook_spacing:50}
 const base=await build({...p,hooks:false}),mounted=await build(p)
 closed(mounted)
 const rectVolume=p.hook_count*p.hook_width*p.hook_thickness*(p.hook_drop+p.hook_thickness+p.hook_gap)
 const r=Math.min(p.hook_thickness*0.85,p.hook_gap*0.35,p.hook_drop*0.35)
 const filletVolume=p.hook_count*p.hook_width*2*(1-2*Math.sqrt(2)/4)*r*r
 expect(mounted.volume-base.volume).toBeGreaterThan(rectVolume)
 expect(Math.abs(mounted.volume-base.volume-(rectVolume+filletVolume))).toBeLessThan(3.5)
 let left=Infinity,right=-Infinity,bottom=Infinity,outside=Infinity
 for(const m of mounted.meshes)for(let i=0;i<m.vertices.length;i+=6){
  if(m.vertices[i+1]<-p.hook_gap-0.01){
   left=Math.min(left,m.vertices[i]);right=Math.max(right,m.vertices[i]);bottom=Math.min(bottom,m.vertices[i+2]);outside=Math.min(outside,m.vertices[i+1])
  }
 }
 expect(left).toBeCloseTo((p.width-p.hook_spacing-p.hook_width)/2,2)
 expect(right-left).toBeCloseTo(p.hook_spacing+p.hook_width,2)
 expect(bottom).toBeCloseTo(p.height-p.hook_drop-p.hook_thickness,2)
 expect(outside).toBeCloseTo(-p.hook_gap-p.hook_thickness,2)
},120000)
it('combines a mesh floor, bend, corner rounds, tilt and custom hooks',async()=>{
 closed(await build({...BOX_DEFAULTS,bend_radius:140,corner_radius:12,front_angle:15,hook_width:12,hook_thickness:4,hook_gap:8,hook_drop:20,hook_spacing:50,floor_cell_size:30}))
},120000)
it('places auto-spaced hooks at Bessel minimum-bending points (~0.2071L) or aligned with dividers',async()=>{
 const bessel=await build({...BOX_DEFAULTS,porous:false,floor_mesh:false,hook_count:2,hook_spacing:0})
 closed(bessel)
 let left=Infinity,right=-Infinity
 for(const m of bessel.meshes)for(let i=0;i<m.vertices.length;i+=6){
  if(m.vertices[i+1]<-BOX_DEFAULTS.hook_gap-0.01){
   left=Math.min(left,m.vertices[i]);right=Math.max(right,m.vertices[i])
  }
 }
 const expectedStep=BOX_DEFAULTS.width*(1-2*0.2071)
 expect(left).toBeCloseTo((BOX_DEFAULTS.width-expectedStep-BOX_DEFAULTS.hook_width)/2,2)
 expect(right-left).toBeCloseTo(expectedStep+BOX_DEFAULTS.hook_width,2)
 // When dividers == hook_count, hooks align directly behind the stiff internal dividers.
 const aligned=await build({...BOX_DEFAULTS,porous:false,floor_mesh:false,dividers:2,hook_count:2,hook_spacing:0})
 closed(aligned)
 let divLeft=Infinity,divRight=-Infinity
 for(const m of aligned.meshes)for(let i=0;i<m.vertices.length;i+=6){
  if(m.vertices[i+1]<-BOX_DEFAULTS.hook_gap-0.01){
   divLeft=Math.min(divLeft,m.vertices[i]);divRight=Math.max(divRight,m.vertices[i])
  }
 }
 const divStep=(BOX_DEFAULTS.width-2*BOX_DEFAULTS.wall)/3
 expect(divLeft).toBeCloseTo((BOX_DEFAULTS.width-divStep-BOX_DEFAULTS.hook_width)/2,2)
 expect(divRight-divLeft).toBeCloseTo(divStep+BOX_DEFAULTS.hook_width,2)
},120000)
it('rejects overlapping hooks and floor ribs that consume the cells',()=>{
 expect(()=>generateBox({...BOX_DEFAULTS,hook_spacing:8})).toThrow(/Крючки/)
 expect(()=>generateBox({...BOX_DEFAULTS,hook_spacing:200})).toThrow(/Крючки/)
 expect(()=>generateBox({...BOX_DEFAULTS,floor_cell_size:1,floor_rib:4})).toThrow(/ячейки дна/)
 expect(()=>generateBox({...BOX_DEFAULTS,floor_cell_size:1,floor_rib:2.4})).not.toThrow()
 expect(()=>generateBox({...BOX_DEFAULTS,hook_width:35})).not.toThrow()
})
