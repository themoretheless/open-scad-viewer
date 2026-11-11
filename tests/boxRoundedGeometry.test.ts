import {beforeAll,it,expect} from 'vitest'
import {GeometryBuildEngine} from '../src/kernel/geometryBuildEngine'
import {extractSemanticEdges} from '../src/kernel/meshTopology'
import {BOX_DEFAULTS,generateBox} from '../src/services/boxGenerator'
const engine=new GeometryBuildEngine()
beforeAll(()=>engine.initializeSource('cube(1);',{quality:'full',purpose:'analysis'}))
it.each([{porous:false,hooks:false,front_angle:0},{porous:false,hooks:false,front_angle:15},{porous:true,hooks:false,front_angle:0},{porous:true,hooks:false,front_angle:15},{porous:true,hooks:true,front_angle:0},{porous:true,hooks:false,front_angle:-15},{porous:true,hooks:true,front_angle:15,dividers:1,drain:true,hole_pitch:24}])('builds rounded walls with configuration %j',async patch=>{
 const {result}=await engine.buildSource(generateBox({...BOX_DEFAULTS,corner_radius:12,...patch}),{quality:'full',purpose:'analysis'})
 expect(result.volume).toBeGreaterThan(0)
 let minBack=Infinity, maxFront=-Infinity
 for(const mesh of result.meshes) {
  for(let i=0;i<mesh.vertices.length;i+=6) if(Math.abs(mesh.vertices[i+2]-BOX_DEFAULTS.height)<0.01 && mesh.vertices[i+1]>=-0.001) {
   minBack=Math.min(minBack,mesh.vertices[i+1]); maxFront=Math.max(maxFront,mesh.vertices[i+1])
  }
  const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
  expect(diagnostics.boundary).toBe(0)
  expect(diagnostics.nonManifold).toBe(0)
 }
 expect(minBack).toBeCloseTo(0,2)
 expect(maxFront).toBeCloseTo(BOX_DEFAULTS.depth+(BOX_DEFAULTS.height-BOX_DEFAULTS.floor)*Math.tan(patch.front_angle*Math.PI/180),1)
},120000)
