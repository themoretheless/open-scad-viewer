import {it,expect} from 'vitest'
import {textureSurface,textureHeight,type SurfaceTextureOptions} from '../src/services/surfaceTexture'
import {extrudeDirectSketch} from '../src/services/directModeling'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
import {solidTopology,facePlane} from '../src/services/directSolidTools'
const body=()=>extrudeDirectSketch({id:'p',name:'Box',closed:true,points:[[0,0],[8,0],[8,8],[0,8]]},8,'0')
const options:SurfaceTextureOptions={pattern:'ribs',pitch:4,height:.2,angle:20,seed:42,detail:3,invert:false,origin:[0,0,0],u:[1,0,0],v:[0,1,0]}
it('generates all six patterns as closed exportable geometry',()=>{for(const pattern of ['ribs','grooves','knurl','fuzzy','dimples','waves'] as const){const b=body(),original=structuredClone(b),r=textureSurface(b,{...options,pattern});expect(r.mesh.indices.length).toBeGreaterThan(b.mesh.indices.length);expect(inspectPolygonMesh(r.mesh).closed).toBe(true);expect(r.mesh.positions).not.toEqual(b.mesh.positions);expect(b).toEqual(original)}})
it('keeps unselected surfaces and border vertices fixed',()=>{const b=body(),face=solidTopology(b.mesh).faces.find(f=>f.normal[2]>.9)!,o={...options,...facePlane(b,face),triangles:face.triangles},r=textureSurface(b,o);for(let i=0;i<r.mesh.positions.length;i+=3){const [x,y,z]=r.mesh.positions.slice(i,i+3);if(z<7.99)expect(z).toBeGreaterThanOrEqual(0);if(x===0||x===8||y===0||y===8)expect(z).toBeLessThanOrEqual(8)}expect(Math.max(...r.mesh.positions.filter((_,i)=>i%3===2))).toBeGreaterThan(8)})
it('has reproducible seed and rejects unsafe size/work budgets',()=>{const b=body(),o={...options,pattern:'fuzzy' as const};expect(textureSurface(b,o)).toEqual(textureSurface(b,o));expect(textureSurface(b,{...o,seed:12}).mesh.positions).not.toEqual(textureSurface(b,o).mesh.positions);expect(()=>textureSurface(b,{...o,height:2})).toThrow('pitch/4');expect(()=>textureSurface(b,{...o,pitch:.01,height:.001})).toThrow('40000')})
it('applies default-sized texture through source editing and the actual parser',async()=>{
 const {parseOpenSCAD}=await import('../src/services/openscadParser'),{sceneBody}=await import('../src/services/mainModeling'),{patchMainSource}=await import('../src/services/mainSourceEditing'),{MAX_WORKSPACE_SOURCE_LENGTH}=await import('../src/services/workspaceDocument')
 const source='cube([20,15,10]);\ntranslate([30,0,0]) cube(4);',meshes=(await parseOpenSCAD(source)).meshes,bodies=meshes.map(sceneBody);bodies[0]=textureSurface(bodies[0],{...options,pitch:6,height:.4,angle:0,detail:3});const text=patchMainSource(source,meshes,{version:1,sketches:[],bodies});expect(text.length).toBeLessThan(MAX_WORKSPACE_SOURCE_LENGTH);expect(text).toContain('translate([30,0,0]) cube(4);');const rebuilt=await parseOpenSCAD(text);expect(rebuilt.meshes).toHaveLength(2);expect(inspectPolygonMesh(sceneBody(rebuilt.meshes[0],0).mesh).closed).toBe(true)
},15000)

it('matches analytic pattern landmarks and legacy unsigned-noise seed fixtures',()=>{
 const o={...options,angle:0}
 for(const [pattern,point,factor] of [
  ['ribs',[0,0,0],1],['ribs',[1,0,0],.25],['grooves',[0,0,0],-1],
  ['knurl',[0,0,0],1],['dimples',[0,0,0],-1],['dimples',[2,2,0],0],['waves',[0,0,0],1],
 ] as const){
  expect(textureHeight([...point],{...o,pattern})).toBeCloseTo(factor*o.height,12)
  expect(textureHeight([...point],{...o,pattern,invert:true})).toBeCloseTo(-factor*o.height,12)
 }
 // Captured from the pre-migration JS hash, including negative cells and wrapped seeds.
 for(const [seed,factor] of [[42,.5357285599595141],[-1,.17853841771337176],[4294967338,.5357285599595141]]){
  expect(textureHeight([-1.5,2.25,-5],{...o,pattern:'fuzzy',seed})).toBeCloseTo(factor*o.height,12)
 }
})
it('refuses stale retained B-rep textures and malformed selections atomically',async()=>{
 const {createBrepBox,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const brep=createBrepBox([0,0,0],[8,8,8]),retained={id:'retained',name:'Stock',brep,mesh:tessellateNurbsBrep(brep,1)},before=JSON.stringify(retained)
 expect(()=>textureSurface(retained,options)).toThrow('retained B-rep')
 expect(JSON.stringify(retained)).toBe(before)
 const b=body(),snapshot=JSON.stringify(b)
 for(const triangles of [[],[-1],[.5],[999]])expect(()=>textureSurface(b,{...options,triangles})).toThrow()
 expect(()=>textureSurface(b,{...options,u:[2,0,0]})).toThrow('frame')
 expect(()=>textureSurface(b,{...options,seed:.5})).toThrow()
 expect(()=>textureHeight([Infinity,0,0],options)).toThrow()
 expect(JSON.stringify(b)).toBe(snapshot)
 expect(inspectPolygonMesh(textureSurface(b,options).mesh).closed).toBe(true)
})
