import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const source = readFileSync('examples/rush/original-self-contact.r','utf8')
const run = async (code: string) => {
  const scene = await buildTextNurbsScene(compileRushFrontend(code).document)
  expect(scene.meshes).toHaveLength(1)
  return new Map(scene.measurements!.map(m=>[m.operation,JSON.parse(m.report)]))
}

it('proves absence and regularity on the original extruded graph', async () => {
  const reports=await run(source)
  expect(reports.get('curve_self_contacts')).toMatchObject({status:'absent',witnessPoint:null})
  expect(reports.get('surface_self_contacts')).toMatchObject({status:'absent',absenceProven:true})
  expect(reports.get('surface_span_regularity')).toMatchObject({spanwiseRegular:true,unresolved:[]})
})

it('returns distinct normalized witnesses for the exact reflected fold and keeps uncertain cells', async () => {
  const reports=await run(source.replace('[1mm,0mm,0mm]', '[0mm,0mm,0mm]'))
  const fold=reports.get('surface_self_contacts')
  expect(fold.status).toBe('present')
  expect(fold.witness[0]).not.toEqual(fold.witness[1])
  const regularity=reports.get('surface_span_regularity')
  expect(regularity.spanwiseRegular).toBe(false)
  expect(regularity.unresolved.length).toBeGreaterThan(0)
  expect(reports.get('curve_self_contacts').status).not.toBe('absent')
})
