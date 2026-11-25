import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/uv-trim-domain.r','utf8')
async function measure(code:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(code).document)
  expect(scene.meshes).toHaveLength(1)
  return new Map(scene.measurements!.map(row=>[row.operation,JSON.parse(row.report)]))
}

it('distinguishes material, an oppositely wound hole and the boundary band',async()=>{
  const reports=await measure(source)
  expect(reports.get('uv_loop_closure').exactJoins).toBe(true)
  expect(reports.get('uv_loop_simplicity').outcome).toBe('ProvenSimple')
  expect(reports.get('uv_trim_location')).toMatchObject({location:'Outside',winding:0})
  const material=await measure(source.replace('u:0.5','u:0.125'))
  expect(material.get('uv_trim_location')).toMatchObject({location:'Inside',winding:1})
  const boundary=await measure(source.replace('u:0.5','u:0.25'))
  expect(boundary.get('uv_trim_location').location).toBe('Unresolved')
})

it('reports an open contour and preserves uncertainty when the pair budget runs out',async()=>{
  const open=await measure(source.replace('sheet.uv_loop_closure(loops:[[outer0,outer1,outer2,outer3','sheet.uv_loop_closure(loops:[[outer0,outer1,outer2'))
  expect(open.get('uv_loop_closure').exactJoins).toBe(false)
  const limited=await measure(source.replace('max_pairs:100','max_pairs:1'))
  expect(limited.get('uv_loop_simplicity')).toMatchObject({provenSimple:false,outcome:'PairBudgetExhausted'})
})
