import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const program=readFileSync('examples/rush/section-basis.r','utf8')
async function measure(source:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return new Map(scene.measurements!.map(row=>[row.operation,JSON.parse(row.report)]))
}

it('elevates the selected linear section, inserts the internal knot and reverses its neighbour',async()=>{
  const r=await measure(program)
  expect(r.get('compatible_section')).toMatchObject({degree:2,knots:[0,0,0,0.5,1,1,1]})
  expect(r.get('oriented_section').orientation).toBe('Reversed')
  const tied=await measure(program.replace('0.001mm','10mm'))
  expect(tied.get('oriented_section').orientation).toBe('Ambiguous')
})

it('rejects an absent section index and preserves the orientation anchor',async()=>{
  const anchor=await measure(program.replace('piece:1','piece:0'))
  expect(anchor.get('oriented_section').orientation).toBe('Anchor')
  await expect(measure(program.replace('piece:1','piece:2'))).rejects.toThrow(/index|section/i)
})
