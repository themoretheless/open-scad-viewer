import {describe,it,expect} from 'vitest'
import {cadOperation,bounds,type CadOptions} from '../src/services/cadWorkbench'
import {extrudeDirectSketch,type DirectDocument} from '../src/services/directModeling'
import {inspectPolygonMesh} from '../src/services/polygonKernel'
import {inspectCadPairs} from '../src/services/cadInspection'
import {cadDrawing} from '../src/services/cadDrawing'
const box=(id='0',x=0)=>extrudeDirectSketch({id:'s',name:'Box',closed:true,points:[[x,0],[x+10,0],[x+10,10],[x,10]]},10,id)
const options:CadOptions={action:'union',ids:['0'],sketches:[],axis:[0,0,1],origin:[0,0,0],amount:5,count:3,width:2,height:1,depth:12,pitch:1,secondary:4,mode:'plain',pathId:'',profileIds:[]}
const doc=(...bodies:ReturnType<typeof box>[]):DirectDocument=>({version:1,sketches:[],bodies})
const volume=(d:DirectDocument)=>d.bodies.reduce((s,b)=>s+inspectPolygonMesh(b.mesh).signedVolumeMm3,0)
describe('CAD workbench geometry',()=>{
 it('booleans preserve selection order and source bodies',()=>{const d=doc(box(),box('1',5));expect(volume(cadOperation(d,{...options,action:'union',ids:['0','1']}))).toBeCloseTo(1500);expect(volume(cadOperation(d,{...options,action:'difference',ids:['1','0']}))).toBeCloseTo(500);expect(volume(d)).toBe(2000)})
 it('mirrors winding, resizes and patterns groups',()=>{expect(volume(cadOperation(doc(box()),{...options,action:'mirror',mode:'replace',axis:[1,0,0]}))).toBeCloseTo(1000);expect(volume(cadOperation(doc(box()),{...options,action:'resize',width:2,height:3,depth:4}))).toBeCloseTo(24);expect(cadOperation(doc(box()),{...options,action:'pattern'}).bodies).toHaveLength(3)})
 it('aligns to first selected body and distributes centers',()=>{const d=doc(box(),box('1',20),box('2',50));const a=cadOperation(d,{...options,action:'align',ids:['1','0'],axis:[1,0,0],mode:'min'});expect(bounds([a.bodies[0]]).min[0]).toBe(20);const b=cadOperation(d,{...options,action:'distribute',ids:['0','1','2'],axis:[1,0,0],mode:'center'});expect(bounds([b.bodies[1]]).min[0]).toBe(25)})
 it('cuts plain/counterbored/countersunk holes and threaded holes',()=>{for(const mode of ['plain','counterbore','countersink']){const d=cadOperation(doc(box()),{...options,action:'hole',origin:[5,5,-1],mode});expect(volume(d)).toBeLessThan(1000);expect(inspectPolygonMesh(d.bodies[0].mesh).closed).toBe(true)}const d=cadOperation(doc(box()),{...options,action:'thread',origin:[5,5,-1],mode:'internal'});expect(volume(d)).toBeLessThan(1000)})
 it('reports intersecting volume and exact separated box gap',()=>{expect(inspectCadPairs([box(),box('1',13)])[0].gapMm).toBeCloseTo(3);expect(inspectCadPairs([box(),box('1',5)])[0].overlapMm3).toBeCloseTo(500)})
 it('produces vector drawings with dimensions and section',()=>{const d=cadDrawing([box()],5);expect(d.svg).toContain('10.00 mm');expect(d.svg).toContain('SECTION Z=5.00');expect(d.pdf).toContain('xref\n0 6');expect(d.pdf).toContain('/Type /Page')})
})

it('round-trips faceted STEP as closed solids and rejects analytic files',async()=>{const {exportFacetedStep,importFacetedStep}=await import('../src/services/cadStep');const text=exportFacetedStep([box(),box('1',20)]),b=importFacetedStep(text);expect(b).toHaveLength(2);expect(volume(doc(...b))).toBeCloseTo(2000);expect(()=>importFacetedStep(text.replace('FACE_SURFACE','ADVANCED_FACE'))).toThrow('supports planar')})
it('lofts sections and sweeps an open path',()=>{const sketches=[{id:'a',name:'a',closed:true,points:[[0,0],[4,0],[4,4],[0,4]] as [number,number][]},{id:'b',name:'b',closed:true,points:[[0,0],[2,0],[2,2],[0,2]] as [number,number][],plane:{origin:[0,0,10] as [number,number,number],u:[1,0,0] as [number,number,number],v:[0,1,0] as [number,number,number]}},{id:'path',name:'path',closed:false,points:[[0,0],[0,10],[10,10]] as [number,number][]}];expect(volume(cadOperation(doc(),{...options,action:'loft',profileIds:['a','b'],sketches}))).toBeGreaterThan(0);expect(volume(cadOperation(doc(),{...options,action:'sweep',profileIds:['a'],pathId:'path',sketches}))).toBeGreaterThan(0)})
it('persists bounded joint positions and applies absolute slider travel after source rebuild',async()=>{
 const {jointOptions,saveCadJoint}=await import('../src/services/cadJoints'),{patchMainSource}=await import('../src/services/mainSourceEditing'),{parseOpenSCAD}=await import('../src/services/openscadParser'),{sceneBody}=await import('../src/services/mainModeling')
 const source='cube(10);translate([20,0,0]) cube(10);',meshes=(await parseOpenSCAD(source)).meshes,before=doc(...meshes.map(sceneBody)),o={...options,action:'joint' as const,ids:['0','1'],mode:'slider',amount:5},d=cadOperation(before,jointOptions(source,before.bodies,o,-10,10)),saved=saveCadJoint(patchMainSource(source,meshes,d),d.bodies,o,-10,10),rebuilt=(await parseOpenSCAD(saved)).meshes.map(sceneBody),next=jointOptions(saved,rebuilt,{...o,amount:8},-10,10)
 expect(next.amount).toBe(3);expect(()=>jointOptions(saved,rebuilt,{...o,amount:11},-10,10)).toThrow('limits');expect(bounds(cadOperation(doc(...rebuilt),next).bodies).max[2]).toBe(18)
})
