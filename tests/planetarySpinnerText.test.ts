import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { planetarySpinnerTemplate } from '../src/services/planetarySpinnerTemplate'
const text=readFileSync('examples/modelgraph-text/planetary-spinner.scad','utf8')
it('preserves the 20 exact spinner meshes through compact text',async()=>{
 const before=await parseOpenSCAD('inner_radius=30.845;outer_radius=32;center_hole_diameter=44;gap=.05;spinner_height=10;helix_angle=35;'+planetarySpinnerTemplate)
 const after=await parseOpenSCAD(text)
 expect(after.meshes).toHaveLength(20)
 after.meshes.forEach((mesh,i)=>{
  expect(Array.from(mesh.vertices)).toEqual(Array.from(before.meshes[i]!.vertices))
  expect(Array.from(mesh.indices)).toEqual(Array.from(before.meshes[i]!.indices))
 })
 expect(compileModelGraphText(text).customizer).toHaveLength(6)
},20000)
it('rejects impossible bore and negative gap',()=>{
 expect(()=>compileModelGraphText(text.replace('44mm','90mm'))).toThrow('Invalid spinner')
 expect(()=>compileModelGraphText(text.replace('0.05mm','-1mm'))).toThrow()
})
