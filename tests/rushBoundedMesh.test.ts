import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/bounded-chord-mesh.r','utf8')
const build=(code:string)=>buildTextNurbsScene(compileRushFrontend(code).document)

it('renders the indexed mesh qualified by Rust and preserves the original chord interval',async()=>{
  const scene=await build(program)
  const reports=new Map(scene.measurements!.map(m=>[m.operation,JSON.parse(m.report)]))
  const chord=reports.get('bounded_curve_chord'), mesh=reports.get('bounded_surface_mesh')
  expect(chord.domain[0]).toBe(0)
  expect(chord.domain[1]).toBeLessThan(1)
  expect(chord.pieces).toBeGreaterThan(1)
  expect(chord.errorUpperMm).toBeLessThanOrEqual(0.25)
  expect(mesh.errorUpperMm).toBeLessThanOrEqual(0.4)
  expect(mesh.stop).toBe('Tolerance')
  expect(scene.meshes[0]!.indices.length).toBe(mesh.triangles*3)
})
it('refuses a missing chord and under-budget meshes before display',async()=>{
  await expect(build(program.replace('piece:0','piece:999'))).rejects.toThrow(/chord/i)
  await expect(build(program.replace('0.4mm,max_cells:1000','0.001mm,max_cells:1'))).rejects.toThrow(/budget|tolerance/i)
})
