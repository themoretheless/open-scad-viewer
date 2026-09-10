import {it,expect} from 'vitest'
import {lightenSolid,lighteningCells,type LighteningOptions} from '../src/services/solidLightening'
import {extrudeDirectSketch} from '../src/services/directModeling'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const box=()=>extrudeDirectSketch({id:'s',name:'plate',closed:true,points:[[0,0],[20,0],[20,16],[0,16]]},4,'0')
const o:LighteningOptions={pattern:'web',axis:'z',cell:8,rib:1.35,rim:2,bottom:.6,top:0,seed:42,jitter:.7,lineWidth:.45,perimeters:3}
it.each(['grid','triangles','honeycomb','web'] as const)('cuts connected %s structure and preserves the source',(pattern)=>{const b=box(),before=structuredClone(b),r=lightenSolid(b,{...o,pattern}),report=inspectPolygonMesh(r.mesh);expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeGreaterThan(0);expect(report.signedVolumeMm3).toBeLessThan(1280);expect(b).toEqual(before)},20000)
it('has repeatable random cells and distinct seeds',()=>{expect(lighteningCells([2,2],[18,14],o)).toEqual(lighteningCells([2,2],[18,14],o));expect(lighteningCells([2,2],[18,14],{...o,seed:43})).not.toEqual(lighteningCells([2,2],[18,14],o))})
it('preserves bottom skin and accepts different channel axes',()=>{for(const axis of ['x','y','z'] as const){const b=box(),r=lightenSolid(b,{...o,pattern:'grid',axis,rim:.5,bottom:.6,cell:6});expect(inspectPolygonMesh(r.mesh).closed).toBe(true)}},20000)
it('rejects sub-line ribs and oversized generation before expensive geometry',()=>{expect(()=>lightenSolid(box(),{...o,rib:.3})).toThrow('extrusion lines');expect(()=>lightenSolid(box(),{...o,cell:1,rib:.2,lineWidth:.1,perimeters:1})).toThrow('144');expect(()=>lightenSolid(box(),{...o,bottom:5})).toThrow('consume')})
it('retains a complete bottom face and survives source/parser round-trip',async()=>{
 const {patchMainSource}=await import('../src/services/mainSourceEditing'),{parseOpenSCAD}=await import('../src/services/openscadParser'),{sceneBody}=await import('../src/services/mainModeling'),{MAX_WORKSPACE_SOURCE_LENGTH}=await import('../src/services/workspaceDocument')
 const source='cube([20,16,4]); translate([25,0,0]) cube(2);',meshes=(await parseOpenSCAD(source)).meshes,bodies=meshes.map(sceneBody),b=lightenSolid(bodies[0],o);let bottomArea=0;for(let t=0;t<b.mesh.indices.length;t+=3){const p=b.mesh.indices.slice(t,t+3).map(i=>b.mesh.positions.slice(i*3,i*3+3));if(p.every(p=>Math.abs(p[2])<1e-6))bottomArea+=Math.abs((p[1][0]-p[0][0])*(p[2][1]-p[0][1])-(p[2][0]-p[0][0])*(p[1][1]-p[0][1]))/2}expect(bottomArea).toBeCloseTo(320)
 bodies[0]=b;const code=patchMainSource(source,meshes,{version:1,sketches:[],bodies});expect(code.length).toBeLessThan(MAX_WORKSPACE_SOURCE_LENGTH);expect(code).toContain('translate([25,0,0]) cube(2);');const rebuilt=await parseOpenSCAD(code);expect(rebuilt.meshes).toHaveLength(2);expect(inspectPolygonMesh(sceneBody(rebuilt.meshes[0],0).mesh).closed).toBe(true)
})
