import {it,expect} from 'vitest'
import {lightenSolid,lighteningCells,type LighteningOptions} from '../src/services/solidLightening'
import {extrudeDirectSketch} from '../src/services/directModeling'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const box=()=>extrudeDirectSketch({id:'s',name:'plate',closed:true,points:[[0,0],[20,0],[20,16],[0,16]]},4,'0')
const o:LighteningOptions={pattern:'web',axis:'z',cell:8,rib:1.35,rim:2,bottom:.6,top:0,seed:42,jitter:.7,lineWidth:.45,perimeters:3}
it('keeps honeycomb sites regular independently of hidden randomization settings',()=>{
 const options={...o,pattern:'honeycomb' as const,cell:6,rib:.2}
 const regular=lighteningCells([0,0],[1,9],{...options,jitter:0})
 expect(lighteningCells([0,0],[1,9],options)).toEqual(regular)
 expect(lighteningCells([0,0],[1,9],{...options,seed:99,jitter:1})).toEqual(regular)
})
it('clips equilateral isogrid openings to the frame and ignores randomization',()=>{
 const options={...o,pattern:'isogrid' as const,cell:6,rib:1.2}
 const cells=lighteningCells([2,2],[18,14],options)
 expect(cells.length).toBeGreaterThan(4)
 expect(cells).toEqual(lighteningCells([2,2],[18,14],{...options,seed:99,jitter:0}))
 for(const polygon of cells)for(const [x,y] of polygon){
  expect(x).toBeGreaterThanOrEqual(2-1e-9);expect(x).toBeLessThanOrEqual(18+1e-9)
  expect(y).toBeGreaterThanOrEqual(2-1e-9);expect(y).toBeLessThanOrEqual(14+1e-9)
 }
 expect(cells.some(p=>{
  if(p.length!==3)return false
  const lengths=p.map((a,i)=>Math.hypot(a[0]-p[(i+1)%3][0],a[1]-p[(i+1)%3][1]))
  return Math.max(...lengths)-Math.min(...lengths)<1e-9
 })).toBe(true)
 for(const cell of [1e-300,.01])expect(()=>lighteningCells([2,2],[18,14],{...options,cell})).toThrow('144')
 expect(()=>lighteningCells([2,2],[1,14],options)).toThrow('bounds')
})
it('returns no openings for consumed cells without trapping the shared kernel',()=>{
 for(const pattern of ['grid','triangles','isogrid'] as const){
  expect(lighteningCells([0,0],[1,1],{...o,pattern,cell:8,rib:2})).toEqual([])
 }
 expect(lighteningCells([0,0],[8,8],{...o,pattern:'grid',cell:8,rib:1})).toHaveLength(1)
})
it.each(['x','y','z'] as const)('cuts isogrid channels along %s with a bottom skin',(axis)=>{
 const result=lightenSolid(box(),{...o,pattern:'isogrid',axis,rim:.5,bottom:.6,cell:6})
 const report=inspectPolygonMesh(result.mesh)
 expect(report.closed).toBe(true);expect(report.degenerateTriangles).toBe(0)
 expect(report.signedVolumeMm3).toBeGreaterThan(0);expect(report.signedVolumeMm3).toBeLessThan(1280)
},20000)
it.each(['grid','triangles','isogrid','honeycomb','web'] as const)('cuts connected %s structure and preserves the source',(pattern)=>{const b=box(),before=structuredClone(b),r=lightenSolid(b,{...o,pattern}),report=inspectPolygonMesh(r.mesh);expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeGreaterThan(0);expect(report.signedVolumeMm3).toBeLessThan(1280);expect(b).toEqual(before)},20000)
it('has repeatable random cells and distinct seeds',()=>{expect(lighteningCells([2,2],[18,14],o)).toEqual(lighteningCells([2,2],[18,14],o));expect(lighteningCells([2,2],[18,14],{...o,seed:43})).not.toEqual(lighteningCells([2,2],[18,14],o))})
it('preserves bottom skin and accepts different channel axes',()=>{for(const axis of ['x','y','z'] as const){const b=box(),r=lightenSolid(b,{...o,pattern:'grid',axis,rim:.5,bottom:.6,cell:6});expect(inspectPolygonMesh(r.mesh).closed).toBe(true)}},20000)
it('rejects sub-line ribs and oversized generation before expensive geometry',()=>{expect(()=>lightenSolid(box(),{...o,rib:.3})).toThrow('extrusion lines');expect(()=>lightenSolid(box(),{...o,cell:1,rib:.2,lineWidth:.1,perimeters:1})).toThrow('144');expect(()=>lightenSolid(box(),{...o,bottom:5})).toThrow('consume')})
it('retains a complete bottom face and survives source/parser round-trip',async()=>{
 const {patchMainSource}=await import('../src/services/mainSourceEditing'),{parseOpenSCAD}=await import('../src/services/openscadParser'),{sceneBody}=await import('../src/services/mainModeling'),{MAX_WORKSPACE_SOURCE_LENGTH}=await import('../src/services/workspaceDocument')
 const source='cube([20,16,4]); translate([25,0,0]) cube(2);',meshes=(await parseOpenSCAD(source)).meshes,bodies=meshes.map(sceneBody),b=lightenSolid(bodies[0],o);let bottomArea=0;for(let t=0;t<b.mesh.indices.length;t+=3){const p=b.mesh.indices.slice(t,t+3).map(i=>b.mesh.positions.slice(i*3,i*3+3));if(p.every(p=>Math.abs(p[2])<1e-6))bottomArea+=Math.abs((p[1][0]-p[0][0])*(p[2][1]-p[0][1])-(p[2][0]-p[0][0])*(p[1][1]-p[0][1]))/2}expect(bottomArea).toBeCloseTo(320)
 bodies[0]=b;const code=patchMainSource(source,meshes,{version:1,sketches:[],bodies});expect(code.length).toBeLessThan(MAX_WORKSPACE_SOURCE_LENGTH);expect(code).toContain('translate([25,0,0]) cube(2);');const rebuilt=await parseOpenSCAD(code);expect(rebuilt.meshes).toHaveLength(2);expect(inspectPolygonMesh(sceneBody(rebuilt.meshes[0],0).mesh).closed).toBe(true)
})
