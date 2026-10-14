import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { compileRushFrontend } from '../src/services/rushFrontend'
const text=readFileSync('examples/rush-frontend/planetary-spinner.r','utf8')
// The one spinner source lives in the runtime crate; Rush expands
// planetary_spinner(...) into it, so the OpenSCAD route must produce the same meshes.
const template=readFileSync('crates/rush-runtime/src/planetary_spinner.scad','utf8')
it('preserves the 20 exact spinner meshes through compact text',async()=>{
 const before=await parseOpenSCAD('$fn=48;inner_radius=30.845;outer_radius=32;center_hole_diameter=44;gap=.05;spinner_height=10;helix_angle=35;'+template)
 const after=await parseOpenSCAD(text)
 expect(after.meshes).toHaveLength(20)
 after.meshes.forEach((mesh,i)=>{
  expect(Array.from(mesh.vertices)).toEqual(Array.from(before.meshes[i]!.vertices))
  expect(Array.from(mesh.indices)).toEqual(Array.from(before.meshes[i]!.indices))
 })
 expect(compileRushFrontend(text).customizer).toHaveLength(6)
},120000)
it('rejects impossible bore and negative gap',()=>{
 expect(()=>compileRushFrontend(text.replace('44mm','90mm'))).toThrow('Отверстие выходит за корень')
 expect(()=>compileRushFrontend(text.replace('0.05mm','-1mm'))).toThrow()
})
