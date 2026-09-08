import {describe,it,expect} from 'vitest'
import {mainOperation,mainSource,primitiveSource,previewMeshes,sceneFace,type MainParameters} from '../src/services/mainModeling'
import {extrudeDirectSketch} from '../src/services/directModeling'
import {inspectPolygonMesh} from '../src/services/polygonKernel'
import type {PickHit} from '../src/services/rendererContracts'
const box=extrudeDirectSketch({id:'s',name:'Box',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]},10,'0')
const meshes=()=>previewMeshes({version:1,sketches:[],bodies:[box]})
const p:MainParameters={amount:2,x:0,y:0,z:0,axis:'z',edge:0,shape:'rectangle',width:2,height:2,cut:false}
const hit:PickHit={meshIndex:0,triangleIndex:0,faceId:0,point:[3,3,0],normal:[0,0,-1],barycentric:[1,0,0],source:null,backside:false}
describe('main viewport editing',()=>{
 it('appends authored primitives rather than copying the scene',()=>{expect(primitiveSource('box',20)).toBe('cube([20,20,20]);');expect(primitiveSource('sphere',4)).toContain('d=4');expect(()=>primitiveSource('box',NaN)).toThrow()})
 it('maps renderer triangle picks to solid faces and applies push/pull without changing the input',()=>{
 const m=meshes(),before=Array.from(m[0].vertices);expect(sceneFace(m[0],hit).face).toBeGreaterThanOrEqual(0)
 const d=mainOperation(m,0,hit,'push',p);expect(inspectPolygonMesh(d.bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1200);expect(Array.from(m[0].vertices)).toEqual(before)
 expect(mainSource(d)).toContain('polyhedron(');expect(previewMeshes(d)[0].indices.length).toBeGreaterThan(0)
 })
 it('rejects missing face picks and performs edge/shell operations on the selected main mesh',()=>{
 expect(()=>mainOperation(meshes(),0,null,'push',p)).toThrow('face')
 expect(inspectPolygonMesh(mainOperation(meshes(),0,hit,'chamfer',p).bodies[0].mesh).signedVolumeMm3).toBeCloseTo(980)
 expect(inspectPolygonMesh(mainOperation(meshes(),0,hit,'shell',p).bodies[0].mesh).signedVolumeMm3).toBeCloseTo(712)
 })
 it('splits, duplicates, deletes and transforms selected bodies',()=>{
 expect(mainOperation(meshes(),0,hit,'split',{...p,amount:5}).bodies).toHaveLength(2)
 expect(mainOperation(meshes(),0,hit,'duplicate',{...p,x:20}).bodies).toHaveLength(2)
 expect(mainOperation(meshes(),0,hit,'delete',p).bodies).toHaveLength(0)
 const d=mainOperation(meshes(),0,hit,'move',{...p,x:20});expect(Math.min(...d.bodies[0].mesh.positions.filter((_,i)=>i%3===0))).toBe(20)
 })
 it('adds and cuts a profile at the actual picked face point',()=>{
 for(const cut of [false,true]){const d=mainOperation(meshes(),0,hit,'profile',{...p,cut});const v=inspectPolygonMesh(d.bodies[0].mesh).signedVolumeMm3;expect(v).toBeCloseTo(cut?992:1008)}
 })
})
