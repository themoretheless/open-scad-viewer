import {beforeAll,it,expect} from 'vitest'
import {GeometryBuildEngine} from '../src/services/geometryBuildEngine'
import {extractSemanticEdges} from '../src/services/meshTopology'
import {BOX_DEFAULTS,generateBox} from '../src/services/boxGenerator'
const engine=new GeometryBuildEngine()
const request={quality:'full',purpose:'analysis'} as const
beforeAll(()=>engine.initializeSource('cube(1);',request),120000)
it.each([false,true])('bends the whole plain shell with reverse=%s and the expected annular radii',async reverse=>{
 const p={...BOX_DEFAULTS,porous:false,hooks:false,floor_mesh:false,bend_radius:200,bend_reverse:reverse}
 const {result}=await engine.buildSource(generateBox(p),request)
 const direction=reverse?-1:1
 const volume=120*75*80-114*69*76
 expect(result.volume/(volume*(1+direction*75/400))).toBeCloseTo(1,2)
 let min=Infinity,max=-Infinity
 for(const mesh of result.meshes) {
  const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
  expect(diagnostics.boundary).toBe(0)
  expect(diagnostics.nonManifold).toBe(0)
  for(let i=0;i<mesh.vertices.length;i+=6) {
   const r=Math.hypot(mesh.vertices[i]-60,mesh.vertices[i+1]+direction*200)
   min=Math.min(min,r);max=Math.max(max,r)
  }
 }
 expect(min).toBeCloseTo(reverse?125:200,2)
 expect(max).toBeCloseTo(reverse?200:275,2)
},120000)
it('bends organic walls, floor and hooks together while retaining front tilt and corner rounds',async()=>{
 const {result}=await engine.buildSource(generateBox({...BOX_DEFAULTS,bend_radius:140,corner_radius:12,front_angle:15}),request)
 expect(result.volume).toBeGreaterThan(0)
 expect(result.meshes.length).toBe(1)
 for(const mesh of result.meshes) {
  const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
  expect(diagnostics.boundary).toBe(0)
  expect(diagnostics.nonManifold).toBe(0)
 }
},120000)
it('rejects wraps and inward bends through the axis',()=>{
 expect(()=>generateBox({...BOX_DEFAULTS,bend_radius:15})).toThrow()
 expect(()=>generateBox({...BOX_DEFAULTS,bend_radius:70,bend_reverse:true})).toThrow()
})

it.each([
 'viewer_bend(radius=0) cube(1);',
 'viewer_bend(radius=-10) cube(1);',
 'viewer_bend(radius=100,step=0) cube(1);',
 'viewer_bend(radius=100) circle(1);',
 'viewer_bend(radius=10) translate([0,-11,0]) cube(1);',
])('rejects invalid bend input: %s',async source=>{
 await expect(engine.buildSource(source,request)).rejects.toThrow()
},120000)
