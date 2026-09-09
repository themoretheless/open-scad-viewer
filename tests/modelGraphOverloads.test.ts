import { expect, it } from 'vitest'
import { compileModelGraph } from '../src/services/modelGraph'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'

const header = '// @modelgraph-text/1\n'
it('accepts three positional dimensions and offsets without changing vector semantics', async () => {
  const compact = header + 'show box(0.01,0.6,0.1).move(4.2,2,54)'
  const vector = header + 'show box([0.01,0.6,0.1]).move([4.2,2,54])'
  expect(compileModelGraphText(compact).source).toBe(compileModelGraphText(vector).source)
  expect((await parseOpenSCAD(compact)).volume).toBeCloseTo(0.0006, 6)
})
it('supports units, defaults and parameter edits in the new forms', () => {
  const source = header + `param width = 2mm range 1mm..8mm
fn part height: length = 4mm => box(width,3mm,height).translate(1cm,0,2mm)
show part()`
  const compiled = compileModelGraphText(source)
  const document = structuredClone(compiled.document)
  document.parameters[0]!.value = 5
  expect(compileModelGraph(document).source).toBe(compileModelGraphText(header+'show box([5mm,3mm,4mm]).move([1cm,0,2mm])').source)
})
it('keeps named and positional-vector calls compatible', () => {
  const variants = ['box(2,3,4).move(1,2,3)', 'box(size:[2,3,4]).move(x:1,y:2,z:3)', 'box([2,3,4]).translate(vector:[1,2,3])']
  expect(new Set(variants.map(source=>compileModelGraphText(header+'show '+source).source)).size).toBe(1)
})
it('moves collections without merging their objects', async () => {
  expect((await parseOpenSCAD(header+'show [box(1,1,1),box(1,1,1).move(1,0,0)].move(0,0,5)')).meshes).toHaveLength(2)
})
it.each(['box(1,2)','box(1,2,3,4)','box([1,2,3],2,3)','box(1,2,size:3)','box(1mm,2deg,3mm)','box(1,0,3)','box(1,2,3).move(1,2)','box(1,2,3).move(1,2,z:3)','box(1,2,3).move(1,2,3deg)'])('rejects invalid overload %s', source=>{
  expect(()=>compileModelGraphText(header+'show '+source)).toThrow()
})
