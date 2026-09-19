import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { compileModelGraphText } from '../src/services/modelGraphText'
const text=readFileSync('examples/modelgraph-text/planetary-spinner.mg','utf8')
// The one spinner source lives in the runtime crate; ModelGraph Text expands
// planetary_spinner(...) into it, so the OpenSCAD route must produce the same meshes.
const template=readFileSync('crates/modelgraph-runtime/src/planetary_spinner.scad','utf8')
it('preserves the 20 exact spinner meshes through compact text',async()=>{
 const before=await parseOpenSCAD('$fn=48;inner_radius=30.845;outer_radius=32;center_hole_diameter=44;gap=.05;spinner_height=10;helix_angle=35;'+template)
 const after=await parseOpenSCAD(text)
 expect(after.meshes).toHaveLength(20)
 after.meshes.forEach((mesh,i)=>{
  expect(Array.from(mesh.vertices)).toEqual(Array.from(before.meshes[i]!.vertices))
  expect(Array.from(mesh.indices)).toEqual(Array.from(before.meshes[i]!.indices))
 })
 expect(compileModelGraphText(text).customizer).toHaveLength(6)
},120000)
it('rejects impossible bore and negative gap',()=>{
 expect(()=>compileModelGraphText(text.replace('44mm','90mm'))).toThrow('Отверстие выходит за корень')
 expect(()=>compileModelGraphText(text.replace('0.05mm','-1mm'))).toThrow()
})
