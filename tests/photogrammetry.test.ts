import {describe,it,expect} from 'vitest'
import {PhotogrammetryKernel} from '../src/services/photogrammetryKernel'
import {photoFocalHint,photoMesh,photoPly,photoCameraFrame,photoCanAppend} from '../src/services/photoReconstruction'
describe('own photogrammetry transport and exports',()=>{
 it('blocks open scan patches from the solid CAD path',()=>{
  const positions=[[0,0,0],[1,0,0],[0,1,0],[0,0,1]],colors=positions.map(()=>[0,0,0]);
  expect(photoCanAppend({positions,colors,triangles:[[0,1,2]]})).toBe(false)
  expect(photoCanAppend({positions,colors,triangles:[[0,2,1],[0,1,3],[1,2,3],[2,0,3]]})).toBe(true)
 })
 it('runs the import-free WASM kernel and rejects blank photos instead of inventing geometry',()=>{const k=new PhotogrammetryKernel();const photo={width:64,height:64,focal:100,rgb:new Uint8Array(64*64*3)};expect(k.add(photo)).toBe(1);expect(k.add(photo)).toBe(2);expect(()=>k.sparse()).toThrow(/initialize/);k.clear();expect(()=>k.dense()).toThrow(/cameras first/);expect(()=>k.compact()).toThrow(/surface first/);expect(()=>k.add({...photo,rgb:new Uint8Array(3)})).toThrow(/dimensions/)})
 it('preserves relative coordinates and colors in PLY while fitting only the preview',()=>{const s={positions:[[0,0,0],[0.001,0,0],[0,0.001,0]],colors:[[255,0,0],[0,255,0],[0,0,255]],triangles:[[0,1,2]]};const before=JSON.stringify(s);const mesh=photoMesh(s);expect(JSON.stringify(s)).toBe(before);expect(mesh.vertices[0]).toBeCloseTo(-50);expect(photoPly(s)).toContain('0.001 0 0 0 255 0');expect(photoPly(s)).toContain('3 0 1 2');expect([...mesh.indices]).toEqual([0,1,2])})
 it('frames from the recovered camera without modifying exported coordinates',()=>{const s={positions:[[-1,-1,5],[1,-1,5],[0,1,5]],colors:[[0,0,0],[0,0,0],[0,0,0]],triangles:[[0,1,2]]};const c={image:0,rotation:[[1,0,0],[0,1,0],[0,0,1]],translation:[0,0,0],focal:500,cx:320,cy:240};const f=photoCameraFrame(s,c,4/3);expect(f.camera.distance).toBeCloseTo(250);expect(f.camera.target).toEqual([-0,0,-0]);expect(f.fovY).toBeGreaterThan(0);expect(s.positions[0]).toEqual([-1,-1,5])})
 it('does not invent focal metadata from arbitrary or truncated data',()=>{expect(photoFocalHint(new ArrayBuffer(0))).toBeNull();expect(photoFocalHint(new Uint8Array([255,216,255,225,0,50,69,120,105,102]).buffer)).toBeNull()})
})
